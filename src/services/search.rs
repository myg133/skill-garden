//! 搜索服务 - Tantivy 全文搜索

use std::path::Path;

use anyhow::Result;
use tantivy::{
    collector::TopDocs,
    query::{BooleanQuery, Occur, QueryParser, TermQuery},
    schema::{Schema, Value, STORED, STRING, TEXT},
    Index, IndexReader, IndexWriter, ReloadPolicy, TantivyDocument,
};
use tracing::{debug, info};
use uuid::Uuid;

use crate::models::error::AppError;
use crate::models::Skill;

/// API Key 搜索 scope
#[derive(Debug, Clone)]
pub enum SearchScope {
    /// 个人 API key：自己的 Skill + 市场已发布
    Personal { identity_id: Uuid },
    /// 组织 API key：该组织的 Skill + 市场已发布
    Organization { org_id: Uuid },
}

/// 搜索结果
#[derive(Debug, Clone, serde::Serialize)]
pub struct SearchResult {
    pub skill_id: String,
    pub score: f32,
    pub install_count: u32,
}

/// 搜索服务
pub struct SearchService {
    index: Index,
    reader: IndexReader,
    schema: Schema,
}

impl std::fmt::Debug for SearchService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SearchService").finish()
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
        std::fs::create_dir_all(index_path)?;

        let mut schema_builder = Schema::builder();
        schema_builder.add_text_field("id", STRING | STORED);
        schema_builder.add_text_field("name", TEXT | STORED);
        schema_builder.add_text_field("description", TEXT | STORED);
        schema_builder.add_text_field("tags", TEXT | STORED);
        schema_builder.add_text_field("content", TEXT);
        schema_builder.add_text_field("install_count", STORED);
        // 可见性过滤字段（STRING = 精确匹配，不分词）
        schema_builder.add_text_field("visibility", STRING | STORED);
        schema_builder.add_text_field("owner_type", STRING | STORED);
        schema_builder.add_text_field("owner_id", STRING | STORED);
        schema_builder.add_text_field("status", STRING | STORED);
        let schema = schema_builder.build();

        let index = if index_path.join("meta.json").exists() {
            Index::open_in_dir(index_path)?
        } else {
            Index::create_in_dir(index_path, schema.clone())?
        };

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

    fn writer(&self) -> Result<IndexWriter, AppError> {
        self.index
            .writer(50_000_000)
            .map_err(|e| AppError::InternalError(e.to_string()))
    }

    /// 索引中文档数量
    pub fn doc_count(&self) -> Result<u64, AppError> {
        let searcher = self.reader.searcher();
        Ok(searcher.num_docs())
    }

    /// 全量重建索引（从数据库加载已发布的 Skill 批量写入）
    pub fn rebuild_from_skills(&self, skills: &[Skill]) -> Result<usize, AppError> {
        let mut writer = self.writer()?;
        // 清空现有索引
        writer.delete_all_documents()?;
        let mut count = 0;
        for skill in skills {
            if skill.status != "published" {
                continue;
            }
            let owner_id_str = skill.owner_id.map(|id| id.to_string()).unwrap_or_default();
            let vis_str = match &skill.visibility {
                crate::models::skill_policy::Visibility::Private => "private",
                crate::models::skill_policy::Visibility::GroupVisible => "group_visible",
                crate::models::skill_policy::Visibility::OrgVisible => "org_visible",
                crate::models::skill_policy::Visibility::TenantVisible => "tenant_visible",
                crate::models::skill_policy::Visibility::Marketplace => "marketplace",
                crate::models::skill_policy::Visibility::Shared => "shared",
            };
            let mut doc = TantivyDocument::new();
            doc.add_text(self.field("id"), &skill.id);
            doc.add_text(self.field("name"), &skill.name);
            doc.add_text(self.field("description"), &skill.description);
            doc.add_text(self.field("tags"), &skill.tags.join(" "));
            doc.add_text(self.field("content"), &skill.content);
            doc.add_text(
                self.field("install_count"),
                &skill.install_count.to_string(),
            );
            doc.add_text(self.field("visibility"), vis_str);
            doc.add_text(self.field("owner_type"), &skill.owner_type);
            doc.add_text(self.field("owner_id"), &owner_id_str);
            doc.add_text(self.field("status"), &skill.status);
            writer.add_document(doc)?;
            count += 1;
        }
        writer.commit()?;
        self.reader.reload()?;
        info!("Rebuilt search index: {} skills indexed", count);
        Ok(count)
    }

    fn field(&self, name: &str) -> tantivy::schema::Field {
        self.schema.get_field(name).expect("missing schema field")
    }

    /// 添加文档到索引
    pub fn add_skill(&self, skill: &Skill) -> Result<(), AppError> {
        let mut writer = self.writer()?;

        let owner_id_str = skill.owner_id.map(|id| id.to_string()).unwrap_or_default();
        let vis_str = match &skill.visibility {
            crate::models::skill_policy::Visibility::Private => "private",
            crate::models::skill_policy::Visibility::GroupVisible => "group_visible",
            crate::models::skill_policy::Visibility::OrgVisible => "org_visible",
            crate::models::skill_policy::Visibility::TenantVisible => "tenant_visible",
            crate::models::skill_policy::Visibility::Marketplace => "marketplace",
            crate::models::skill_policy::Visibility::Shared => "shared",
        };

        let mut doc = TantivyDocument::new();
        doc.add_text(self.field("id"), &skill.id);
        doc.add_text(self.field("name"), &skill.name);
        doc.add_text(self.field("description"), &skill.description);
        doc.add_text(self.field("tags"), &skill.tags.join(" "));
        doc.add_text(self.field("content"), &skill.content);
        doc.add_text(
            self.field("install_count"),
            &skill.install_count.to_string(),
        );
        doc.add_text(self.field("visibility"), vis_str);
        doc.add_text(self.field("owner_type"), &skill.owner_type);
        doc.add_text(self.field("owner_id"), &owner_id_str);
        doc.add_text(self.field("status"), &skill.status);

        writer.add_document(doc)?;
        writer.commit()?;
        self.reader.reload()?;

        debug!("Indexed skill: {}", skill.id);
        Ok(())
    }

    /// 从索引中删除文档
    pub fn delete_skill(&self, skill_id: &str) -> Result<(), AppError> {
        let mut writer = self.writer()?;
        let term = tantivy::Term::from_field_text(self.field("id"), skill_id);
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

    /// 构建 scope 过滤查询
    fn scope_filter(&self, scope: Option<&SearchScope>) -> Option<BooleanQuery> {
        let scope = scope?;
        let (owner_type_val, owner_id_val) = match scope {
            SearchScope::Personal { identity_id } => ("user", identity_id.to_string()),
            SearchScope::Organization { org_id } => ("organization", org_id.to_string()),
        };

        // status:published → 只搜索已发布的 Skill
        let status_term = TermQuery::new(
            tantivy::Term::from_field_text(self.field("status"), "published"),
            tantivy::schema::IndexRecordOption::Basic,
        );

        // visibility:marketplace → 市场已发布 Skill 所有人可见
        let marketplace_term = TermQuery::new(
            tantivy::Term::from_field_text(self.field("visibility"), "marketplace"),
            tantivy::schema::IndexRecordOption::Basic,
        );

        // owner_type:xxx AND owner_id:yyy → 个人/组织 Skill
        let owner_type_term = TermQuery::new(
            tantivy::Term::from_field_text(self.field("owner_type"), owner_type_val),
            tantivy::schema::IndexRecordOption::Basic,
        );
        let owner_id_term = TermQuery::new(
            tantivy::Term::from_field_text(self.field("owner_id"), &owner_id_val),
            tantivy::schema::IndexRecordOption::Basic,
        );

        // (owner_type:xxx AND owner_id:yyy AND status:published) OR (visibility:marketplace AND status:published)
        let owner_and = BooleanQuery::new(vec![
            (Occur::Must, Box::new(owner_type_term)),
            (Occur::Must, Box::new(owner_id_term)),
            (Occur::Must, Box::new(status_term.clone())),
        ]);

        let marketplace_and = BooleanQuery::new(vec![
            (Occur::Must, Box::new(marketplace_term)),
            (Occur::Must, Box::new(status_term)),
        ]);

        let filter = BooleanQuery::new(vec![
            (Occur::Should, Box::new(owner_and)),
            (Occur::Should, Box::new(marketplace_and)),
        ]);

        Some(filter)
    }

    /// 搜索（带 scope 过滤）
    pub fn search(
        &self,
        query_str: &str,
        tags: Option<&[String]>,
        limit: usize,
        scope: Option<&SearchScope>,
    ) -> Result<Vec<SearchResult>, AppError> {
        let searcher = self.reader.searcher();
        let id_field = self.field("id");
        let install_field = self.field("install_count");

        let query_parser = QueryParser::for_index(
            &self.index,
            vec![
                self.field("name"),
                self.field("description"),
                self.field("tags"),
                self.field("content"),
            ],
        );

        let mut subqueries: Vec<(Occur, Box<dyn tantivy::query::Query>)> = Vec::new();

        // 关键词查询
        if !query_str.is_empty() {
            match query_parser.parse_query(query_str) {
                Ok(q) => subqueries.push((Occur::Must, Box::new(q))),
                Err(e) => {
                    tracing::warn!("Query parse error for '{}': {}", query_str, e);
                    return Ok(vec![]);
                }
            }
        }

        // 标签过滤
        if let Some(tag_list) = tags {
            for tag in tag_list {
                if !tag.is_empty() {
                    let term = tantivy::Term::from_field_text(self.field("tags"), tag);
                    subqueries.push((
                        Occur::Must,
                        Box::new(TermQuery::new(
                            term,
                            tantivy::schema::IndexRecordOption::Basic,
                        )),
                    ));
                }
            }
        }

        // scope 过滤
        if let Some(filter) = self.scope_filter(scope) {
            subqueries.push((Occur::Must, Box::new(filter)));
        }

        let query: Box<dyn tantivy::query::Query> = if subqueries.is_empty() {
            Box::new(tantivy::query::AllQuery)
        } else {
            Box::new(BooleanQuery::new(subqueries))
        };

        let top_docs = searcher.search(&query, &TopDocs::with_limit(limit))?;

        let mut results = Vec::with_capacity(top_docs.len());
        for (_score, doc_address) in top_docs {
            let doc: TantivyDocument = searcher.doc(doc_address)?;
            let skill_id = doc
                .get_first(id_field)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let install_count = doc
                .get_first(install_field)
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);
            results.push(SearchResult {
                skill_id,
                score: _score,
                install_count,
            });
        }

        debug!("Search '{}' returned {} results", query_str, results.len());
        Ok(results)
    }
}
