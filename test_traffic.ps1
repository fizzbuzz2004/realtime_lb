# test_traffic.ps1 - Generiert kontinuierlichen Traffic & zeigt Prometheus-Metriken an

Param(
    [string]$TargetUrl = "http://127.0.0.1:8080/",
    [string]$MetricsUrl = "http://127.0.0.1:8080/metrics",
    [int]$DelayMs = 200,          # Pause zwischen Anfragen in Millisekunden
    [int]$MetricsEvery = 10       # Metriken nach jeweils X Requests anzeigen
)

Clear-Host
Write-Host "==================================================" -ForegroundColor Cyan
Write-Host "   Real-Time Load Balancer Traffic Generator      " -ForegroundColor Cyan
Write-Host "==================================================" -ForegroundColor Cyan
Write-Host "Ziel-URL     : $TargetUrl" -ForegroundColor Gray
Write-Host "Metrics-URL  : $MetricsUrl" -ForegroundColor Gray
Write-Host "Abbruch mit  : STRG + C`n" -ForegroundColor Yellow

$i = 0

while ($true) {
    $i++
    
    # 1. Anfrage an den Load Balancer senden
    try {
        $startTime = Get-Date
        $response = Invoke-RestMethod -Uri $TargetUrl -Method Get -ErrorAction Stop
        $elapsed = [math]::Round(((Get-Date) - $startTime).TotalMilliseconds, 2)
        
        Write-Host "[$i] HTTP 200 OK (${elapsed}ms) -> $response" -ForegroundColor Green
    }
    catch {
        Write-Host "[$i] ERROR -> $_" -ForegroundColor Red
    }

    # 2. Periodisch Prometheus-Metriken auslesen und formatiert ausgeben
    if ($i % $MetricsEvery -eq 0) {
        Write-Host "`n--------------------------------------------------" -ForegroundColor DarkCyan
        Write-Host " PROMETHEUS METRICS SNAPSHOT (Request #$i)" -ForegroundColor DarkCyan
        Write-Host "--------------------------------------------------" -ForegroundColor DarkCyan
        
        try {
            # Verwende curl.exe für verlässlichen Plain-Text Empfang
            $metricsRaw = curl.exe -s$MetricsUrl
            
            if ($metricsRaw) {$metricsRaw -split "`n" | ForEach-Object {
                    $line = $_.Trim()
                    if ($line -and -not $line.StartsWith("#")) {
                        if ($line.StartsWith("lb_current_strategy")) {
                            Write-Host "  Strategy Active : $line" -ForegroundColor Yellow
                        }
                        elseif ($line.StartsWith("lb_requests_total")) {
                            Write-Host "  Total Requests  : $line" -ForegroundColor White
                        }
                        elseif ($line.StartsWith("lb_backend_healthy")) {
                            Write-Host "  Health Status   : $line" -ForegroundColor Gray
                        }
                        elseif ($line.StartsWith("lb_backend_latency_ms")) {
                            Write-Host "  Avg Latency     : $line" -ForegroundColor DarkYellow
                        }
                        else {
                            Write-Host "  $line" -ForegroundColor DarkGray
                        }
                    }
                }
            } else {
                Write-Host "  Leere Antwort von $MetricsUrl" -ForegroundColor Red
            }
        }
        catch {
            Write-Host "  Fehler beim Abrufen von $MetricsUrl" -ForegroundColor Red
        }
        
        Write-Host "--------------------------------------------------`n" -ForegroundColor DarkCyan
    }

    Start-Sleep -Milliseconds $DelayMs
}
