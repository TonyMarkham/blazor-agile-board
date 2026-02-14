# Global Instructions

## pm CLI operations

When doing pm CLI work (creating work items, updating statuses, listing, etc.), use haiku sub-agents via the Task tool with `model: "haiku"` and `subagent_type: "Bash"` to minimize cost. Reserve Opus/Sonnet for planning and complex reasoning — not for running `./pm` commands.
