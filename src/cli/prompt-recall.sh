#!/bin/sh
# Attach recalled project context to a prompt. Claude Code hands this hook the
# UserPromptSubmit event on stdin and reads additionalContext off stdout.
#
# Every failure here is silent and exit 0 on purpose: recall is an addition to a
# prompt, so a missing daemon, an unbuilt host or an empty record must cost the
# prompt nothing. The prompt itself is never touched.
set -u

root=${CLAUDE_PROJECT_DIR:-$(pwd)}
event=$(cat) || exit 0

command -v cartridge >/dev/null 2>&1 || exit 0
command -v jq >/dev/null 2>&1 || exit 0

prompt=$(printf '%s' "$event" | jq -r 'if (.prompt | type) == "string" then .prompt else empty end' 2>/dev/null) || exit 0
[ -n "$prompt" ] || exit 0

request=$(jq -n --arg cwd "$root" --arg query "$prompt" \
    '{op:"context",action:"recall",cwd:$cwd,query:$query}' 2>/dev/null) || exit 0

answer=$(cd "$root" && cartridge run memo "$request" 2>/dev/null) || exit 0
[ -n "$answer" ] || exit 0

block=$(printf '%s' "$answer" | jq -r 'if (.block | type) == "string" then .block else empty end' 2>/dev/null) || exit 0
[ -n "$block" ] || exit 0

jq -n --arg context "$block" \
    '{hookSpecificOutput:{hookEventName:"UserPromptSubmit",additionalContext:$context}}'
