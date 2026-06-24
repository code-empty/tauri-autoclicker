Write-Host "Downloading Rustup installer..." -ForegroundColor Cyan
$url = "https://win.rustup.rs/x86_64"
$output = "rustup-init.exe"
Invoke-WebRequest -Uri $url -OutFile $output
Write-Host "Download complete. Installing Rust (rustup-init -y)..." -ForegroundColor Cyan
Start-Process -FilePath ".\$output" -ArgumentList "-y" -Wait
Remove-Item ".\$output"
Write-Host "Rust installed successfully!" -ForegroundColor Green
Write-Host "Important: Please make sure Windows C++ Build Tools are installed. You can download them from https://visualstudio.microsoft.com/visual-cpp-build-tools/ and select Desktop development with C++." -ForegroundColor Yellow
Write-Host "Please restart your terminal/IDE for PATH changes to take effect." -ForegroundColor Yellow
