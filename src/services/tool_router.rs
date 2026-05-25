//! Tool Router Service - Routes tool calls to appropriate targets

use crate::models::session::{ToolRouter, RouteTarget};

/// ToolRouterService handles tool call routing decisions.
/// It determines whether a tool call should go to:
/// - Local (agent's own implementation)
/// - Platform (built-in platform tools like browse, qa, exec)
/// - Org (organization-registered CLI tools)
#[derive(Debug, Clone)]
pub struct ToolRouterService {
    /// Platform tool IDs (always available)
    platform_tools: Vec<String>,
}

impl ToolRouterService {
    pub fn new() -> Self {
        Self {
            platform_tools: vec![
                "browse".to_string(),
                "qa".to_string(),
                "exec".to_string(),
                "storage".to_string(),
            ],
        }
    }

    /// Determine which target should handle a tool call
    pub fn route_tool(&self, tool_id: &str, org_tools: &[String]) -> RouteTarget {
        // Check if it's a platform tool
        if self.platform_tools.contains(&tool_id.to_string()) {
            return RouteTarget::Platform;
        }

        // Check if it's an org tool
        if org_tools.contains(&tool_id.to_string()) {
            return RouteTarget::OrgTool(tool_id.to_string());
        }

        // Default to local (agent's own implementation)
        RouteTarget::Local
    }

    /// Build a complete routing table for an agent
    pub fn build_routing_table(
        &self,
        agent_capabilities: &[String],
        org_tools: &[String],
    ) -> ToolRouter {
        let mut router = ToolRouter::new();

        // Platform tools always route to platform
        for tool in &self.platform_tools {
            router.add_route(tool.clone(), RouteTarget::Platform);
        }

        // Agent capabilities route to local
        for cap in agent_capabilities {
            if !self.platform_tools.contains(cap) {
                router.add_route(cap.clone(), RouteTarget::Local);
            }
        }

        // Org tools route to org
        for tool in org_tools {
            if !self.platform_tools.contains(tool) {
                router.add_route(tool.clone(), RouteTarget::OrgTool(tool.clone()));
            }
        }

        router
    }

    /// Check if a tool is a platform tool
    pub fn is_platform_tool(&self, tool_id: &str) -> bool {
        self.platform_tools.contains(&tool_id.to_string())
    }

    /// Check if a tool is an org tool
    #[allow(dead_code)]
    pub fn is_org_tool(&self, tool_id: &str, org_tools: &[String]) -> bool {
        org_tools.contains(&tool_id.to_string())
    }
}

impl Default for ToolRouterService {
    fn default() -> Self {
        Self::new()
    }
}
