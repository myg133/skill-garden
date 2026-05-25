param([int]$Port, [string]$DataDir)
$env:AION_HIVE_TRANSPORT = "http"
$env:AION_HIVE_HTTP_PORT = $Port.ToString()
$env:AION_HIVE_DATA_DIR = $DataDir
$env:RUST_LOG = "error"
$psi = New-Object System.Diagnostics.ProcessStartInfo
$psi.FileName = "target\debug\aion-hive.exe"
$psi.UseShellExecute = $false
$psi.EnvironmentVariables["AION_HIVE_TRANSPORT"] = "http"
$psi.EnvironmentVariables["AION_HIVE_HTTP_PORT"] = $Port.ToString()
$psi.EnvironmentVariables["AION_HIVE_DATA_DIR"] = $DataDir
$psi.EnvironmentVariables["RUST_LOG"] = "error"
$process = [System.Diagnostics.Process]::Start($psi)
$process.Id