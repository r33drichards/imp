# Fix imp-cli Supervisor Permission Error

## Problem
The imp-cli supervisor program is failing with "Permission denied (os error 13)" when running as a non-root user.

## Current Configuration
```ini
[program:imp-cli]
command=/usr/local/bin/imp-cli apply --config /mnt/persist/imp-config.toml
priority=15
autostart=true
autorestart=false
startsecs=0
startretries=0
exitcodes=0
stdout_logfile=/var/log/imp-cli.out.log
stderr_logfile=/var/log/imp-cli.err.log
directory=/home/gem
user=gem
```

## Error
```
Error: Permission denied (os error 13)
```

## Root Cause
imp-cli uses bind mounts for directories, which require root/sudo privileges. The current configuration runs as user `gem` without elevated permissions, causing mount operations to fail.

## Requirements
1. Fix the permission error so imp-cli can create bind mounts
2. Maintain proper security practices (don't run everything as root unnecessarily)
3. Ensure the solution works with supervisor
4. Keep logs in /var/log/imp-cli.{out,err}.log
5. Preserve the working directory as /home/gem
6. Maintain autostart behavior

## Preferred Solution Options (pick the best for this infrastructure)

### Option 1: Run Supervisor as Root with User Context Switching
Run the supervisor program as root but use sudo to switch context for state directory access.

### Option 2: Passwordless Sudo for gem User
Configure sudoers to allow gem user to run imp-cli without password, then update supervisor to use sudo.

### Option 3: Convert to Systemd Service
Replace supervisor with a systemd oneshot service that runs at boot (most appropriate for system-level operations).

## Deliverables
1. Updated supervisor configuration OR systemd service file
2. Any additional configuration files needed (sudoers, etc.)
3. Verification steps to confirm the fix works
4. Documentation explaining the chosen approach

## Technical Context
- imp-cli is a persistence manager that uses bind mounts for directories
- Bind mounts require CAP_SYS_ADMIN capability (effectively root)
- The gem user needs access to /home/gem for state directory
- Config file is at /mnt/persist/imp-config.toml
- Default state directory is ~/.local/share/imp (relative to running user)

## References
- imp-cli requires sudo for apply/switch commands (creates bind mounts)
- imp-cli list/verify commands do NOT require sudo (read-only)
- Recent fix (commit 0e01118) handles bind mount ownership by copying from source to target
