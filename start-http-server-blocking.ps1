param([int]$Port = 8080, [string]$DataDir = "test-data")
$env:AION_HIVE_TRANSPORT = "http"
$env:AION_HIVE_HTTP_PORT = $Port.ToString()
$env:AION_HIVE_DATA_DIR = $DataDir
$env:RUST_LOG = "error"
cmd /c "target\debug\aion-hive.exe"