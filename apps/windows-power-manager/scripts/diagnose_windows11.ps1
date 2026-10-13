# Zilla Power Manager diagnostics (READ ONLY).
# Windows PowerShell 5.1 must detect the UTF-8 BOM to correctly decode Cyrillic.
# Windows 11 PowerShell 5.1 or 7; does not change power plans, displays,
# BIOS, clocks, battery controller, GPU limits, or registry.
param(
    [string]$OutputPath = [System.IO.Path]::Combine(
        [Environment]::GetFolderPath('Desktop'),
        'Zilla_Diagnostic_Windows11.txt'
    )
)
$ErrorActionPreference = 'Continue'
$lines = New-Object 'System.Collections.Generic.List[string]'

function Add-Line([string]$Value) {
    $script:lines.Add($Value)
}
function Add-Section([string]$Title) {
    Add-Line ""
    Add-Line ("==== " + $Title + " ====")
}
function Add-Result([string]$Name, [scriptblock]$Read) {
    Add-Section $Name
    try {
        $result = & $Read | Out-String -Width 200
        if (-not [string]::IsNullOrWhiteSpace($result)) {
            Add-Line $result.TrimEnd()
        } else {
            Add-Line "(Порожня відповідь)"
        }
    } catch {
        Add-Line ("Помилка: " + $_.Exception.Message)
    }
}

Add-Line "Zilla Power Manager - Windows hardware and display diagnosis"
Add-Line ("Дата: " + (Get-Date -Format "yyyy-MM-dd HH:mm:ss"))
Add-Line "Режим: лише читання; налаштування системи не змінюються."
Add-Line "Звіт не містить серійних номерів, імені ПК чи користувача."

Add-Result "Операційна система" {
    Get-CimInstance Win32_OperatingSystem -ErrorAction Stop |
        Select-Object Caption, Version, BuildNumber, OSArchitecture
}

Add-Result "Процесор" {
    Get-CimInstance Win32_Processor -ErrorAction Stop |
        Select-Object Name, NumberOfCores, NumberOfLogicalProcessors, MaxClockSpeed
}

Add-Result "Зведення сокетів CPU і логічних потоків" {
    $processors = @(Get-CimInstance Win32_Processor -ErrorAction Stop)
    [PSCustomObject]@{
        PhysicalProcessors = $processors.Count
        Cores = ($processors | Measure-Object -Property NumberOfCores -Sum).Sum
        LogicalProcessors = ($processors | Measure-Object -Property NumberOfLogicalProcessors -Sum).Sum
        MaxClockMHz = ($processors | Measure-Object -Property MaxClockSpeed -Maximum).Maximum
    } | Format-List
}

Add-Result "Оперативна пам'ять (кількість модулів, без serial)" {
    $modules = @(Get-CimInstance Win32_PhysicalMemory -ErrorAction Stop)
    [PSCustomObject]@{
        Modules = $modules.Count
        TotalGB = [math]::Round((($modules | Measure-Object -Property Capacity -Sum).Sum / 1GB), 1)
        SpeedsMHz = (($modules | Where-Object { $_.Speed -gt 0 } | Select-Object -ExpandProperty Speed -Unique) -join ",")
    } | Format-List
}

Add-Result "Відеокарти Windows" {
    Get-CimInstance Win32_VideoController -ErrorAction Stop |
        Select-Object Name, VideoProcessor, DriverVersion, Status, CurrentHorizontalResolution, CurrentVerticalResolution, CurrentRefreshRate
}

Add-Result "Активні дисплеї (номери та головний)" {
    Add-Type -AssemblyName System.Windows.Forms -ErrorAction Stop
    [System.Windows.Forms.Screen]::AllScreens |
        ForEach-Object {
            [PSCustomObject]@{
                DeviceName = $_.DeviceName
                Primary    = $_.Primary
                X          = $_.Bounds.X
                Y          = $_.Bounds.Y
                Width      = $_.Bounds.Width
                Height     = $_.Bounds.Height
                Bits       = $_.BitsPerPixel
            }
        } | Format-Table -AutoSize
}

Add-Result "Моделі моніторів (без серійників)" {
    Get-CimInstance -Namespace root\wmi -ClassName WmiMonitorID -ErrorAction Stop |
        ForEach-Object {
            $chars = @($_.UserFriendlyName | Where-Object { $_ -ne 0 })
            $model = -join ($chars | ForEach-Object { [char]$_ })
            [PSCustomObject]@{
                Model = $model
                Active = $_.Active
            }
        } | Format-Table -AutoSize
}

Add-Result "Активний план живлення" {
    powercfg /getactivescheme
}
Add-Result "CPU max processor state (AC/DC)" {
    powercfg /query SCHEME_CURRENT SUB_PROCESSOR PROCTHROTTLEMAX
}
Add-Result "Автоматичний сон (AC/DC)" {
    powercfg /query SCHEME_CURRENT SUB_SLEEP STANDBYIDLE
}
Add-Result "Автоматична гібернація (AC/DC)" {
    powercfg /query SCHEME_CURRENT SUB_SLEEP HIBERNATEIDLE
}
Add-Result "Автоматичне вимкнення дисплеїв (AC/DC)" {
    powercfg /query SCHEME_CURRENT SUB_VIDEO VIDEOIDLE
}

Add-Result "LibreHardwareMonitor - чи запущений процес" {
    Get-Process -Name LibreHardwareMonitor,OpenHardwareMonitor -ErrorAction SilentlyContinue |
        Select-Object ProcessName, Id, MainWindowTitle
}

Add-Result "Датчики GPU/CPU, якщо працює LibreHardwareMonitor WMI" {
    try {
        $all = @(Get-CimInstance -Namespace 'root\LibreHardwareMonitor' -ClassName Sensor -ErrorAction Stop)
        $chosen = @($all | Where-Object {
            $_.SensorType -in @('Power', 'Load', 'Temperature') -and
            ($_.Identifier -match '/gpu-|/cpu|/intelcpu|/amdcpu|/generic')
        })
        if ($chosen.Count -eq 0) {
            Write-Output "WMI доступний, але не віддає GPU/CPU сенсори. Перевір версію LibreHardwareMonitor."
        } else {
            $chosen | Select-Object -First 100 Name, SensorType, Value, Identifier |
                Format-Table -AutoSize
        }
    } catch {
        Write-Output ("WMI недоступний: " + $_.Exception.Message)
        Write-Output "Потрібен запущений LibreHardwareMonitor з робочим WMI; деякі версії мають відомий збій WMI."
    }
}

Add-Result "Додаткова інформація GPU: доступні AMD-процеси" {
    Get-Process -Name RadeonSoftware,AMDRSServ,amdow -ErrorAction SilentlyContinue |
        Select-Object ProcessName, Id | Format-Table -AutoSize
}

Add-Section "Пояснення конфігурації моніторів"
Add-Line "Логічний Primary відповідає налаштуванням Windows і може бути іншим, ніж фізично бажаний головний."
Add-Line "Win32_VideoController.CurrentRefreshRate не є перевіркою частоти кожного окремого дисплея."
Add-Line "Для визначення фактичних моніторів скористайся Windows Settings > System > Display > Identify."
Add-Line "Вимкнення DDC/CI лишає логічний дисплей активним. Не використовуйте фізичне вимкнення для зняття екрану з робочого столу."

Add-Section "Пояснення ватів"
Add-Line "GPU Board Power / CPU Package Power - лише відповідні компоненти, а не всі вати інвертора."
Add-Line "Для цілі 150-200 Вт потрібен зовнішній ватметр 230 В або перевірений DC-лічильник інвертора/BMS."
Add-Line "Windows не бачить заряд зовнішнього 4S LiFePO4 KEMOT як laptop battery."

try {
    $folder = Split-Path -Path $OutputPath -Parent
    if (-not [string]::IsNullOrWhiteSpace($folder) -and -not (Test-Path -LiteralPath $folder)) {
        New-Item -ItemType Directory -Path $folder -Force | Out-Null
    }
    $lines | Set-Content -LiteralPath $OutputPath -Encoding UTF8 -ErrorAction Stop
    Write-Host ("Звіт готовий: " + $OutputPath) -ForegroundColor Green
} catch {
    Write-Error ("Не вдалося зберегти файл: " + $_.Exception.Message)
    $lines | Out-String | Write-Output
    exit 1
}
