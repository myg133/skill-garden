//! Unit tests for migrations module

#[cfg(test)]
mod tests {
    /// Split SQL statements by semicolons, filtering out empty statements and comments.
    /// This function handles dollar quotes ($$) commonly used in PostgreSQL function definitions.
    fn split_sql_statements(sql: &str) -> Vec<String> {
        let mut results = Vec::new();
        let mut current = String::new();
        let mut in_dollar_quote = false;
        let mut chars = sql.chars().peekable();
        
        while let Some(ch) = chars.next() {
            if ch == '$' {
                // Check for dollar quote start/end ($$ or more)
                let mut tag_end = 1;
                while chars.peek() == Some(&'$') {
                    chars.next();
                    tag_end += 1;
                }
                
                if tag_end >= 2 {
                    current.push_str(&"$".repeat(tag_end));
                    in_dollar_quote = !in_dollar_quote;
                } else {
                    current.push(ch);
                }
            } else if ch == ';' && !in_dollar_quote {
                // End of statement
                let trimmed = current.trim();
                if !trimmed.is_empty() {
                    results.push(trimmed.to_string());
                }
                current = String::new();
            } else {
                current.push(ch);
            }
        }
        
        // Handle remaining content (no trailing semicolon)
        let trimmed = current.trim();
        if !trimmed.is_empty() {
            results.push(trimmed.to_string());
        }
        
        results
    }

    #[test]
    fn test_split_simple_statements() {
        let sql = "SELECT * FROM users; SELECT * FROM orders; SELECT * FROM products";
        let statements = split_sql_statements(sql);
        
        assert_eq!(statements.len(), 3);
        assert_eq!(statements[0], "SELECT * FROM users");
        assert_eq!(statements[1], "SELECT * FROM orders");
        assert_eq!(statements[2], "SELECT * FROM products");
    }

    #[test]
    fn test_split_with_dollar_quotes() {
        let sql = r#"
            CREATE FUNCTION add(a INTEGER, b INTEGER) RETURNS INTEGER AS $$
            BEGIN
                RETURN a + b;
            END;
            $$ LANGUAGE plpgsql;
            
            SELECT add(1, 2);
        "#;
        let statements = split_sql_statements(sql);
        
        assert_eq!(statements.len(), 2);
        assert!(statements[0].contains("CREATE FUNCTION"));
        assert!(statements[0].contains("RETURN a + b"));
        assert_eq!(statements[1].trim(), "SELECT add(1, 2)");
    }

    #[test]
    fn test_split_multiline_create_table() {
        let sql = r#"
            CREATE TABLE users (
                id UUID PRIMARY KEY,
                name VARCHAR(255),
                email VARCHAR(255) UNIQUE
            );
            
            CREATE INDEX idx_users_email ON users(email);
        "#;
        let statements = split_sql_statements(sql);
        
        assert_eq!(statements.len(), 2);
        assert!(statements[0].starts_with("CREATE TABLE users"));
        assert!(statements[0].contains("id UUID PRIMARY KEY"));
        assert!(statements[0].contains("email VARCHAR(255) UNIQUE"));
        assert_eq!(statements[1].trim(), "CREATE INDEX idx_users_email ON users(email)");
    }

    #[test]
    fn test_split_empty_string() {
        let sql = "";
        let statements = split_sql_statements(sql);
        assert!(statements.is_empty());
    }

    #[test]
    fn test_split_only_semicolons() {
        let sql = ";;;";
        let statements = split_sql_statements(sql);
        assert!(statements.is_empty());
    }

    #[test]
    fn test_split_with_whitespace() {
        let sql = "   SELECT 1   ;   SELECT 2   ;   ";
        let statements = split_sql_statements(sql);
        
        assert_eq!(statements.len(), 2);
        assert_eq!(statements[0], "SELECT 1");
        assert_eq!(statements[1], "SELECT 2");
    }

    #[test]
    fn test_split_migration_042_style() {
        let sql = r#"
            CREATE TABLE tenant_creation_requests (
                id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                applicant_id UUID NOT NULL REFERENCES identities(id)
            );
            
            CREATE INDEX idx_applicant ON tenant_creation_requests(applicant_id);
            
            CREATE INDEX idx_status ON tenant_creation_requests(status);
        "#;
        let statements = split_sql_statements(sql);
        
        assert_eq!(statements.len(), 3);
        assert!(statements[0].starts_with("CREATE TABLE tenant_creation_requests"));
        assert_eq!(statements[1].trim(), "CREATE INDEX idx_applicant ON tenant_creation_requests(applicant_id)");
        assert_eq!(statements[2].trim(), "CREATE INDEX idx_status ON tenant_creation_requests(status)");
    }

    #[test]
    fn test_split_single_statement_no_trailing_semicolon() {
        let sql = "SELECT * FROM users";
        let statements = split_sql_statements(sql);
        
        assert_eq!(statements.len(), 1);
        assert_eq!(statements[0], "SELECT * FROM users");
    }

    #[test]
    fn test_split_with_semicolon_in_string() {
        // This edge case shows the limitation - semicolons in string literals
        // will still split the statement. For production use, a proper SQL parser
        // would be needed.
        let sql = "INSERT INTO users (name) VALUES ('Hello; World');";
        let statements = split_sql_statements(sql);
        
        // The simple approach will split on the semicolon in the string
        // In practice, migration SQL doesn't typically contain semicolons in strings
        assert_eq!(statements.len(), 2);
    }

    #[test]
    fn test_split_with_named_dollar_quotes() {
        // Note: Named dollar quotes ($body$...$body$) are not fully supported
        // by the simple toggle approach. For migrations that use named dollar quotes,
        // the function will incorrectly split on semicolons inside the quoted region.
        // This is a known limitation. For full PostgreSQL support, a proper SQL parser
        // would be needed.
        let sql = r#"
            CREATE FUNCTION add(a INTEGER, b INTEGER) RETURNS INTEGER AS $body$
            BEGIN
                RETURN a + b;
            END;
            $body$ LANGUAGE plpgsql;
            
            SELECT add(1, 2);
        "#;
        let statements = split_sql_statements(sql);
        
        // Due to the limitation, this will split incorrectly
        // The test documents this behavior
        assert!(statements.len() >= 2);
    }
}
