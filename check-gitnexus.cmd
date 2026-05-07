@echo off
powershell -NoProfile -ExecutionPolicy Bypass -NoExit -File "%~dp0scripts\gitnexus.ps1" check %*
