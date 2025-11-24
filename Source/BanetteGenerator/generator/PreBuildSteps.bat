@echo off
setlocal

REM ============ 1) 解析 VS 根目录（优先 VSROOT） ============
if "%VSROOT%" == "" (
  set "VSROOT=C:\Program Files\Microsoft Visual Studio\2022\Community"
) 

if not exist "%VSROOT%\VC\Auxiliary\Build\vcvars64.bat" (
  echo [ERROR] Cannot find vcvars64.bat under:
  echo         "%VSROOT%\VC\Auxiliary\Build\vcvars64.bat"
  echo         Please set VSROOT to your Visual Studio install root.
  exit /b 1
)

echo Using Visual Studio at: "%VSROOT%"

REM ============ 2) 初始化 VS 构建环境 ============
call "%VSROOT%\VC\Auxiliary\Build\vcvars64.bat"
if errorlevel 1 (
  echo [ERROR] vcvars64.bat failed
  exit /b 1
)

REM ============ 3) Rust 工具链 & 构建 ============
set "RUSTUP_TOOLCHAIN=stable-x86_64-pc-windows-msvc"

cd /d "%~dp0"  || exit /b 1

cargo build --release --target x86_64-pc-windows-msvc || exit /b 1
cargo build --target x86_64-pc-windows-msvc || exit /b 1

echo Done.
exit /b 0
