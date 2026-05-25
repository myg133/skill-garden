<#
.SYNOPSIS
    Seed admin agent for AionHive admin panel.

.DESCRIPTION
    Registers an admin-1 agent via the API (if not exists) and ensures
    the admin role is applied. The admin agent is used to log into the
    admin panel at /login.

    Requires the backend to be running in HTTP mode on port 8081.
#>

param(
    [string]$Port = "8081",
    [string]$AgentId = "admin-1",
    [string]$AgentName = "Admin Agent"
)

$BaseUrl = "http://localhost:$Port/api"

# Register the admin agent
Write-Host "Registering agent '$AgentId'..."
try {
    $resp = Invoke-RestMethod -Uri "$BaseUrl/agents/register" -Method POST `
        -ContentType "application/json" `
        -Body (ConvertTo-Json @{ agent_id = $AgentId; agent_name = $AgentName }) `
        -ErrorAction Stop

    Write-Host "  Agent registered successfully." -ForegroundColor Green
    Write-Host "  Secret: $($resp.secret)" -ForegroundColor Yellow
    Write-Host "  IMPORTANT: Save this secret - it will not be shown again."
} catch {
    if ($_.Exception.Message -match "already exists") {
        Write-Host "  Agent '$AgentId' already exists." -ForegroundColor Yellow
    } else {
        Write-Host "  ERROR: $($_.Exception.Message)" -ForegroundColor Red
        exit 1
    }
}

# Apply admin role via direct DB update (migration handles this, but run manually too)
Write-Host "Ensuring admin role is set..."
$env:DATABASE_URL = "postgres://postgres@localhost:5432/aionhive"
& psql -c "UPDATE agents SET roles = ARRAY['admin'], updated_at = NOW() WHERE agent_id = '$AgentId' AND NOT ('admin' = ANY(roles));" 2>$null

Write-Host ""
Write-Host "Admin agent ready:" -ForegroundColor Green
Write-Host "  Agent ID:   $AgentId"
Write-Host "  Login at:    http://localhost:5174/login"
Write-Host ""
