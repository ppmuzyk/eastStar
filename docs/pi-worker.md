# Pi Worker Notes

`pi` is available on this machine and is a good fit for local coding-worker flows.

## Useful Modes

- One-shot task: `pi -p "task"`
- Structured event stream: `pi --mode json "task"`
- Headless integration over JSONL stdin/stdout: `pi --mode rpc`

## Recommended Project Usage

Run Pi from the repository root so it picks up the local `AGENTS.md`.

Examples:

```bash
cd /home/ppmuzyk/Projects/eastStar
pi -p "Review the current architecture and suggest the next module to implement"
pi --mode json "Summarize the repo status"
```

If project trust matters in non-interactive runs, add `--approve`.

## Why It Fits Here

- It auto-loads project `AGENTS.md`.
- It can run one-shot jobs without a UI.
- It exposes JSON and RPC modes for later automation.

## Future Harness Idea

If we want a custom OpenClaw worker bridge later, the cleanest first path is:

1. start with `pi -p` for simple detached jobs
2. switch to `pi --mode json` for richer status
3. move to `pi --mode rpc` only if we need full interactive orchestration
