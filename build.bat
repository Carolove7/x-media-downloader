@echo off
chcp 65001 >nul
REM ============================================================
REM  X媒体下载器 (Tauri/Rust 版) 一键构建脚本
REM  依赖: Node.js + Rust (本机已装 GNU 工具链 + mingw64)
REM  产物: C:\Users\19641\.rust-target\release\x-media-downloader.exe
REM         并自动复制到本项目 release\ 目录
REM ============================================================
cd /d "%~dp0"

REM ---- 环境设置（针对本机已配置的工具链）----
REM mingw64: dlltool/as/gcc/ld (GNU 工具链编译 windows-sys 必需)
set "MINGW=C:\Users\19641\mingw64"
if exist "%MINGW%\bin\dlltool.exe" set "PATH=%MINGW%\bin;%PATH%"
REM 关键: dlltool 不支持中文路径, 编译目标目录必须指向纯 ASCII 路径
set "CARGO_TARGET_DIR=C:\Users\19641\.rust-target"

echo [1/3] 检查工具链...
where cargo >nul 2>nul
if errorlevel 1 (
    echo [错误] 未找到 cargo。请先安装 Rust: https://rustup.rs
    pause
    exit /b 1
)

echo [2/3] 构建前端...
call npm install || goto :fail
call npm run build || goto :fail

echo [3/3] 编译 Rust (release, 首次约需 3-8 分钟)...
cd src-tauri
cargo build --release || goto :fail

echo [4/4] 复制产物...
if not exist "..\release" mkdir "..\release"
copy /y "%CARGO_TARGET_DIR%\release\x-media-downloader.exe" "..\release\" >nul

REM ---- WebView2Loader.dll (GNU 工具链构建必需; 若改用 MSVC 工具链则无需此文件) ----
if not exist "..\release\WebView2Loader.dll" (
    for /f "delims=" %%i in ('dir /s /b "%USERPROFILE%\.cargo\registry\src\*webview2-com-sys*\x64\WebView2Loader.dll" 2^>nul') do (
        copy /y "%%i" "..\release\" >nul
        echo [提示] 已复制 WebView2Loader.dll ^(GNU 构建必需^)。
    )
)

if exist "..\release\config.json" (
    echo [提示] 已保留 release\config.json, 配置沿用。
) else if exist "%~dp0..\..\X媒体下载器\dist\config.json" (
    copy /y "%~dp0..\..\X媒体下载器\dist\config.json" "..\release\" >nul
    echo [提示] 已从旧版复制 config.json。
)

echo.
echo ============================================================
echo  构建成功: %~dp0release\x-media-downloader.exe  (约 6MB)
echo  release\ 目录须含 3 个文件: exe + WebView2Loader.dll + config.json
echo  自检(不开窗口, 验证 Cookie/接口/下载):
echo      x-media-downloader.exe --selftest
echo      结果写入 release\selftest_result.txt
echo ============================================================
pause
exit /b 0

:fail
echo.
echo [错误] 构建失败, 请检查上方日志。
pause
exit /b 1
