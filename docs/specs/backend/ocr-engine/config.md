# Config Module Spec

## Function: `effective_max_concurrent_files(config) -> usize`

Calculates the maximum number of files that can be processed concurrently.

```
if config.max_concurrent_files is Some(value):
    return max(value, 1)
else:
    return max(available_parallelism / 2, 1)
```

## Acceptance Criteria
- Explicit value is clamped to minimum 1
- Default (None) returns half of available parallelism
- Default always returns at least 1
