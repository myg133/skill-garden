//! 工具模块

pub mod cli_token;
pub mod rate_limiter;
pub mod weight;

pub use rate_limiter::*;
pub use weight::*;

/// 将字符串转换为 URL 友好的 slug
/// 例如: "My Tenant Name" -> "my-tenant-name"
pub fn slugify(text: &str) -> String {
    text.to_lowercase()
        .chars()
        .map(|c| {
            if c.is_alphanumeric() {
                c
            } else if c.is_whitespace() {
                '-'
            } else {
                '-' // 将其他字符替换为连字符
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<&str>>()
        .join("-")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slugify_basic() {
        assert_eq!(slugify("My Tenant"), "my-tenant");
    }

    #[test]
    fn test_slugify_with_special_chars() {
        assert_eq!(slugify("Acme Corp!"), "acme-corp");
    }

    #[test]
    fn test_slugify_already_slug() {
        assert_eq!(slugify("already-slug"), "already-slug");
    }

    #[test]
    fn test_slugify_chinese() {
        // 中文字符会被转为连字符
        assert_eq!(slugify("我的租户"), "-------");
    }
}
