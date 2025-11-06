# Auto-Fix CI Failures

This repository uses [mini-agent-action](https://github.com/r33drichards/mini-agent-action) to automatically detect and fix CI failures.

## How It Works

When CI checks fail on **main branch or pull requests**, the auto-fix workflow automatically:

1. **Detects failures**: Monitors the CI workflow for failed jobs (fmt, clippy, or test)
2. **Runs mini-agent-action**: Launches an AI agent to analyze and fix the specific failures
3. **Creates a draft PR**: Opens a draft pull request with the automated fixes

## Supported Fix Types

The auto-fix system handles three types of failures:

### 1. Formatting Issues (`cargo fmt`)
- **Detection**: When `Format Check` job fails
- **Fix Command**: `cargo fmt --check`
- **Task**: Runs `cargo fmt` to format all Rust code according to project standards

### 2. Clippy Warnings (`cargo clippy`)
- **Detection**: When `Clippy` job fails
- **Fix Command**: `cargo clippy -- -D warnings`
- **Task**: Analyzes and fixes clippy warnings in the source code

### 3. Test Failures (`cargo test`)
- **Detection**: When `Tests` job fails
- **Fix Command**: `cargo test`
- **Task**: Debugs and fixes failing unit tests

## Setup Requirements

### Required Secret

Add the following secret to your repository settings:

**`ANTHROPIC_API_KEY`**: Your Anthropic API key for Claude
- Get one at: https://console.anthropic.com/
- Go to: Settings → Secrets and variables → Actions → New repository secret

### Permissions

The workflow requires these permissions (already configured):
- `contents: write` - To create branches and commit fixes
- `pull-requests: write` - To create draft PRs
- `actions: read` - To read failed job information

## Workflow Behavior

### When Auto-Fix Triggers

The workflow triggers automatically when CI fails on:
- Pushes to the `main` branch
- Pull requests (from any branch)

### When Auto-Fix Doesn't Run

The workflow will NOT run if:
- CI passes successfully
- The workflow is not from main branch or a pull request (e.g., feature branches)
- No changes are made by the agent (already fixed)
- The `ANTHROPIC_API_KEY` secret is not configured

## Using the Auto-Fix PR

When a draft PR is created:

1. **Review the changes**: Carefully examine what the agent fixed
2. **Check CI status**: Ensure all checks pass on the fix branch
3. **Merge or close**:
   - ✅ Merge if fixes are correct
   - ❌ Close if you prefer to fix manually

### PR Details

Each auto-fix PR includes:
- Clear title indicating it's an automated fix
- List of which checks failed and were fixed
- Link to the original failed workflow run
- Commit SHA of the failure

## Examples

### Example 1: Formatting Fix

```
Failed: cargo fmt --check
Agent: Runs cargo fmt
Result: PR with all files properly formatted
```

### Example 2: Clippy Warning

```
Failed: cargo clippy -- -D warnings
Agent: Analyzes warnings, modifies code to fix issues
Result: PR with clippy-compliant code
```

### Example 3: Test Failure

```
Failed: cargo test
Agent: Debugs test output, fixes implementation or test code
Result: PR with passing tests
```

## Limitations

- The agent may not always fix complex issues correctly
- Review all automated changes before merging
- Some failures may require human intervention
- Integration tests are not auto-fixed (too complex)

## Disabling Auto-Fix

To disable auto-fix, either:
1. Delete `.github/workflows/auto-fix.yml`
2. Remove the `ANTHROPIC_API_KEY` secret
3. Add `[skip auto-fix]` to your commit message

## Troubleshooting

### Agent fails to make changes
- Check that `ANTHROPIC_API_KEY` is set correctly
- Review the workflow logs for error messages
- The issue might be too complex for automated fixing

### PR not created
- Verify repository permissions are correct
- Check that the agent actually made changes
- Review workflow logs for git/gh errors

### Changes don't fix the issue
- Close the PR and fix manually
- The agent's context may have been insufficient
- Consider opening an issue with the failure details

## Credits

This auto-fix system uses [mini-agent-action](https://github.com/r33drichards/mini-agent-action), which adapts Mini SWE Agent to implement automated code fixes with a bash tool that loops until task completion.
