//! 搜索服务 - Tantivy 全文搜索

use std::path::Path;

use anyhow::Result;
use tantivy::{
    collector::TopDocs,
    query::QueryParser,
    schema::{Schema, STORED, STRING, TEXT, Value},
    Index, IndexReader, IndexWriter, ReloadPolicy, TantivyDocument,
};
use tracing::{debug, info};

use crate::models::error::AppError;
use crate::models::Skill;

/// 搜索服务
pub struct SearchService {
    index: Index,
    reader: IndexReader,
    schema: Schema,
}

impl std::fmt::Debug for SearchService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SearchService")
            .finish()
    }
}

impl Clone for SearchService {
    fn clone(&self) -> Self {
        Self {
            index: self.index.clone(),
            reader: self.reader.clone(),
            schema: self.schema.clone(),
        }
    }
}

impl SearchService {
    /// 创建新的搜索服务
    pub fn new(index_path: &Path) -> Result<Self> {
        // 确保索引目录存在
        std::fs::create_dir_all(index_path)?;

        // 构建 schema
        let mut schema_builder = Schema::builder();
        schema_builder.add_text_field("id", STRING | STORED);
        schema_builder.add_text_field("name", TEXT | STORED);
        schema_builder.add_text_field("description", TEXT | STORED);
        schema_builder.add_text_field("tags", TEXT | STORED);
        schema_builder.add_text_field("content", TEXT); // 不存储，减少索引大小
        schema_builder.add_text_field("install_count", STORED); // 存储为文本便于显示
        let schema = schema_builder.build();

        // 创建或打开索引
        let index = if index_path.join("meta.json").exists() {
            Index::open_in_dir(index_path)?
        } else {
            Index::create_in_dir(index_path, schema.clone())?
        };

        // 创建 reader
        let reader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::OnCommitWithDelay)
            .try_into()?;

        info!("Search service initialized at {:?}", index_path);

        Ok(Self {
            index,
            reader,
            schema,
        })
    }

    /// 获取 IndexWriter
    pub fn writer(&self) -> Result<IndexWriter, AppError> {
        self.index
            .writer(50_000_000) // 50MB heap
            .map_err(|e| AppError::InternalError(e.to_string()))
    }

    /// 添加文档到索引
    pub fn add_skill(&self, skill: &Skill) -> Result<(), AppError> {
        let mut writer = self.writer()?;

        let id_field = self.schema.get_field("id").unwrap();
        let name_field = self.schema.get_field("name").unwrap();
        let desc_field = self.schema.get_field("description").unwrap();
        let tags_field = self.schema.get_field("tags").unwrap();
        let content_field = self.schema.get_field("content").unwrap();
        let install_field = self.schema.get_field("install_count").unwrap();

        let mut doc = TantivyDocument::new();
        doc.add_text(id_field, &skill.id);
        doc.add_text(name_field, &skill.name);
        doc.add_text(desc_field, &skill.description);
        doc.add_text(tags_field, &skill.tags.join(" "));
        doc.add_text(content_field, &skill.content);
        doc.add_text(install_field, &skill.install_count.to_string());

        writer.add_document(doc)?;
        writer.commit()?;

        self.reader.reload()?;

        debug!("Indexed skill: {}", skill.id);

        Ok(())
    }

    /// 从索引中删除文档
    pub fn delete_skill(&self, skill_id: &str) -> Result<(), AppError> {
        let mut writer = self.writer()?;

        let id_field = self.schema.get_field("id").unwrap();
        let term = tantivy::Term::from_field_text(id_field, skill_id);
        writer.delete_term(term);
        writer.commit()?;

        self.reader.reload()?;

        debug!("Deleted skill from index: {}", skill_id);

        Ok(())
    }

    /// 更新文档（先删后加）
    pub fn update_skill(&self, skill: &Skill) -> Result<(), AppError> {
        self.delete_skill(&skill.id)?;
        self.add_skill(skill)
    }

    /// 搜索
    pub fn search(
        &self,
        query_str: &str,
        tags: Option<&[String]>,
        limit: usize,
    ) -> Result<Vec<SearchResult>, AppError> {
        let searcher = self.reader.searcher();

        let name_field = self.schema.get_field("name").unwrap();
        let desc_field = self.schema.get_field("description").unwrap();
        let tags_field = self.schema.get_field("tags").unwrap();
        let content_field = self.schema.get_field("content").unwrap();

        let query_parser = QueryParser::for_index(
            &self.index,
            vec![name_field, desc_field, tags_field, content_field],
        );

        // 构建查询
        let mut query_string = query_str.to_string();

        // 添加标签过滤
        if let Some(tag_list) = tags {
            if !tag_list.is_empty() {
                let tag_query = tag_list
                    .iter()
                    .map(|t| format!("tags:{}", t))
                    .collect::<Vec<_>>()
                    .join(" OR ");
                query_string = format!("({}) AND ({})", query_string, tag_query);
            }
        }

        let query = query_parser
            .parse_query(&query_string)
            .map_err(|e| AppError::ValidationError(e.to_string()))?;

        let top_docs = searcher
            .search(&query, &TopDocs::with_limit(limit))
            .map_err(|e| AppError::InternalError(e.to_string()))?;

        let id_field = self.schema.get_field("id").unwrap();
        let install_field = self.schema.get_field("install_count").unwrap();

        let mut results = Vec::new();
        for (score, doc_address) in top_docs {
            if let Ok(doc) = searcher.doc::<TantivyDocument>(doc_address) {
                let id = doc
                    .get_first(id_field)
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
                    .unwrap_or_default();
                let install_count = doc
                    .get_first(install_field)
                    .and_then(|v| v.as_str())
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0);

                results.push(SearchResult {
                    skill_id: id,
                    score,
                    install_count,
                });
            }
        }

        Ok(results)
    }

    /// 列出所有索引的 skill IDs
    #[allow(dead_code)]
    pub fn list_all(&self) -> Result<Vec<String>, AppError> {
        let searcher = self.reader.searcher();
        let id_field = self.schema.get_field("id").unwrap();

        let mut ids = Vec::new();
        for segment_reader in searcher.segment_readers() {
            let store_reader = segment_reader.get_store_reader(1)?;
            for doc_id in 0..segment_reader.max_doc() {
                if let Ok(doc) = store_reader.get::<TantivyDocument>(doc_id) {
                    if let Some(id_value) = doc.get_first(id_field) {
                        if let Some(id_str) = id_value.as_str() {
                            ids.push(id_str.to_string());
                        }
                    }
                }
            }
        }

        Ok(ids)
    }

    /// 重建整个索引
    pub fn rebuild_index(&self, skills: Vec<Skill>) -> Result<(), AppError> {
        let mut writer = self.writer()?;

        // 清空现有索引
        writer.delete_all_documents()?;
        writer.commit()?;

        // 重新索引所有 skills
        let id_field = self.schema.get_field("id").unwrap();
        let name_field = self.schema.get_field("name").unwrap();
        let desc_field = self.schema.get_field("description").unwrap();
        let tags_field = self.schema.get_field("tags").unwrap();
        let content_field = self.schema.get_field("content").unwrap();
        let install_field = self.schema.get_field("install_count").unwrap();

        for skill in skills {
            let mut doc = TantivyDocument::new();
            doc.add_text(id_field, &skill.id);
            doc.add_text(name_field, &skill.name);
            doc.add_text(desc_field, &skill.description);
            doc.add_text(tags_field, &skill.tags.join(" "));
            doc.add_text(content_field, &skill.content);
            doc.add_text(install_field, &skill.install_count.to_string());

            writer.add_document(doc)?;
        }

        writer.commit()?;

        self.reader.reload()?;

        info!("Rebuilt search index");

        Ok(())
    }
}

/// 搜索结果
#[derive(Debug, Clone, serde::Serialize)]
pub struct SearchResult {
    pub skill_id: String,
    pub score: f32,
    pub install_count: u32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Visibility;
    use tempfile::TempDir;

    fn create_test_skill(id: &str, name: &str, description: &str, tags: Vec<&str>) -> Skill {
        Skill {
            id: id.to_string(),
            name: name.to_string(),
            description: description.to_string(),
            tags: tags.into_iter().map(|s| s.to_string()).collect(),
            version: "1.0.0".to_string(),
            author_agent_id: "agent-test".to_string(),
            author_identity_id: None,
            owner_type: "user".to_string(),
            owner_id: None,
            created: chrono::Utc::now(),
            updated: chrono::Utc::now(),
            compatibility: ">=1.0.0".to_string(),
            dependencies: vec![],
            content: format!("# {} Content", name),
            install_count: 10,
            git_url: None,
            visibility: Visibility::OrgVisible,
            tools: vec![],
            review_status: "approved".to_string(),
            reviewed_by: None,
            reviewed_at: None,
            review_comment: None,
        }
    }

    #[test]
    fn test_search_service_debug() {
        let temp_dir = TempDir::new().unwrap();
        let search = SearchService::new(temp_dir.path()).unwrap();
        let debug_str = format!("{:?}", search);
        assert!(debug_str.contains("SearchService"));
    }

    #[test]
    fn test_search_service_clone() {
        let temp_dir = TempDir::new().unwrap();
        let search = SearchService::new(temp_dir.path()).unwrap();
        let _cloned = search.clone();
    }

    #[test]
    fn test_search_result_debug() {
        let result = SearchResult {
            skill_id: "test".to_string(),
            score: 1.0,
            install_count: 5,
        };
        let debug_str = format!("{:?}", result);
        assert!(debug_str.contains("test"));
    }

    #[test]
    fn test_search_result_serde() {
        let result = SearchResult {
            skill_id: "test".to_string(),
            score: 0.95,
            install_count: 10,
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("\"skill_id\":\"test\""));
        assert!(json.contains("\"install_count\":10"));
    }

    #[test]
    fn test_search_with_no_results() {
        let temp_dir = TempDir::new().unwrap();
        let search = SearchService::new(temp_dir.path()).unwrap();

        let results = search.search("nonexistent_query_xyz", None, 10).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_search_with_install_count() {
        let temp_dir = TempDir::new().unwrap();
        let search = SearchService::new(temp_dir.path()).unwrap();

        let skill = create_test_skill("skill-install-test-1.0.0", "install-test", "Testing install count", vec!["test"]);
        search.add_skill(&skill).unwrap();

        let results = search.search("install", None, 10).unwrap();
        assert!(!results.is_empty());
        assert_eq!(results[0].install_count, 10);
    }

    #[test]
    fn test_rebuild_index() {
        let temp_dir = TempDir::new().unwrap();
        let search = SearchService::new(temp_dir.path()).unwrap();

        let new_skills = vec![
            create_test_skill("skill-rebuild-new-1-1.0.0", "rebuild-new-1", "New skill after rebuild", vec!["new"]),
        ];
        search.rebuild_index(new_skills).unwrap();

        let results_new = search.search("rebuild", None, 10).unwrap();
        assert_eq!(results_new.len(), 1);
    }

    #[test]
    fn test_list_all() {
        let temp_dir = TempDir::new().unwrap();
        let search = SearchService::new(temp_dir.path()).unwrap();

        let skills = vec![
            create_test_skill("skill-list-1-1.0.0", "list-1", "List test 1", vec!["test"]),
            create_test_skill("skill-list-2-1.0.0", "list-2", "List test 2", vec!["test"]),
        ];

        search.add_skill(&skills[0]).unwrap();
        search.add_skill(&skills[1]).unwrap();

        let ids = search.list_all().unwrap();
        assert!(ids.contains(&"skill-list-1-1.0.0".to_string()));
        assert!(ids.contains(&"skill-list-2-1.0.0".to_string()));
    }

    #[test]
    fn test_update_skill() {
        let temp_dir = TempDir::new().unwrap();
        let search = SearchService::new(temp_dir.path()).unwrap();

        let skill = create_test_skill("skill-update-1.0.0", "update-test", "Original description", vec!["test"]);
        search.add_skill(&skill).unwrap();

        let mut updated_skill = skill.clone();
        updated_skill.description = "Updated description".to_string();
        updated_skill.tags = vec!["updated".to_string()];

        search.update_skill(&updated_skill).unwrap();

        let results = search.search("Updated", None, 10).unwrap();
        assert!(!results.is_empty());
    }
}
