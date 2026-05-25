$env:AION_HIVE_TRANSPORT = "http"
$env:AION_HIVE_HTTP_PORT = $args[0]
$env:AION_HIVE_DATA_DIR = $args[1]
$env:RUST_LOG = "error"
& "target\debug\aion-hive.exe"