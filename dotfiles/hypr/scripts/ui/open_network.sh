#!/usr/bin/env bash
# Open msUI Network & Wi-Fi Management Panel

msui wifi 2>/dev/null || nm-connection-editor &
