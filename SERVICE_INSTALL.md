# Starting the service at boot (systemd)

`mk/service.mk` + `systemd/service.service.in` install this project as a
systemd unit. The pair is **generic**: nothing in either file knows what this
project is or that it uses Docker. The unit only runs make goals in this
folder:

```ini
ExecStart=/usr/bin/make -C /path/to/project start
ExecStop=/usr/bin/make -C /path/to/project stop
ExecReload=/usr/bin/make -C /path/to/project start
```

So the Makefile stays the single source of truth: change how the service starts
by changing the `start` goal, not the unit. To reuse this in another project,
copy `mk/service.mk` and `systemd/service.service.in`, include the former from
its Makefile, and make sure that Makefile has the required goals.

## Required goals

| Goal | Required | Used for |
|------|----------|----------|
| `start` | yes | `ExecStart` — bring the service up and return |
| `stop` | yes | `ExecStop` |
| `start` (again) | — | `ExecReload`, unless `SERVICE_RELOAD_GOAL` names another |
| `status` | no | `make service-status` prints it when the goal exists |

In this repository they are defined next to `docker-build`/`docker-run` in the
Makefile, and wrap `docker compose up -d` / `down` / `ps`.

Check before installing — it fails with guidance if a goal is missing:

```bash
make service-check
```

If the project names its goals differently, no need to add new ones; point the
installer at the existing ones:

```bash
make service-install SERVICE_START_GOAL=docker-up SERVICE_STOP_GOAL=docker-down
```

## Commands

```bash
make init-system        # is systemd in charge of this machine?
make service-check      # are the required goals present?
make service-show       # print the unit that would be installed (no sudo)
make service-install    # install, enable and start it (sudo)
make service-status     # unit state + the project's own status goal
make service-logs       # the unit's own log: its start and stop runs
make service-uninstall  # stop, disable, remove the unit (sudo)
```

After installing, drive the service through systemd rather than calling the
make goals by hand, so systemd's view of it stays correct:

```bash
sudo systemctl start|stop|restart testcase-manager
systemctl status testcase-manager
```

## Variables

Override on the command line, or assign in the including Makefile *before* the
include (`mk/service.mk` uses `?=`, so the first definition wins):

| Variable | Default | Meaning |
|----------|---------|---------|
| `SERVICE` | directory name | unit name, `/etc/systemd/system/$(SERVICE).service` |
| `SERVICE_DESCRIPTION` | `$(SERVICE) (started by make in ...)` | `Description=` |
| `SERVICE_START_GOAL` | `start` | goal for `ExecStart` |
| `SERVICE_STOP_GOAL` | `stop` | goal for `ExecStop` |
| `SERVICE_RELOAD_GOAL` | the start goal | goal for `ExecReload` |
| `SERVICE_STATUS_GOAL` | `status` | goal shown by `make service-status` |
| `SERVICE_REQUIRES` | `docker.service` | `Requires=`; empty for none |
| `SERVICE_AFTER` | `docker.service network-online.target` | `After=` |
| `SERVICE_ENABLE_UNITS` | `docker.service` | also `systemctl enable`d at install |
| `SERVICE_TIMEOUT_SEC` | `1800` | `TimeoutStartSec=`, for slow first starts |
| `SERVICE_TEMPLATE` | `systemd/service.service.in` | unit template path |

This repository sets `SERVICE := testcase-manager` above the include.

For a project with no Docker in it:

```bash
make service-install SERVICE_REQUIRES= SERVICE_AFTER=network-online.target SERVICE_ENABLE_UNITS=
```

## Which init system does this machine use?

```bash
make init-system
```

It prints PID 1, what `/sbin/init` resolves to, and whether this installer
applies. By hand:

```bash
ps -p 1 -o comm=                              # systemd → systemd; init → SysV
readlink -f /sbin/init                        # /lib/systemd/systemd on systemd
test -d /run/systemd/system && echo systemd   # the reliable check
systemctl is-system-running                   # fails outright if systemd isn't PID 1
```

`/run/systemd/system` is the one to trust: it exists only when systemd is
actually running the machine, not merely installed.

**Ubuntu has shipped systemd as default since 15.04**, so on any current Ubuntu
(including 24.04) the answer is systemd. `/etc/init.d` still existing does not
mean the host uses SysV init — systemd keeps running those scripts through
`systemd-sysv-generator`.

## Notes on the unit

`Type=oneshot` with `RemainAfterExit=yes`: the start goal returns once the
service is up, and the flag keeps the unit "active" afterwards so that
`systemctl stop` has something to stop. This assumes a `start` goal that
*returns* (`docker compose up -d`, not a foreground process). A goal that
blocks in the foreground would want `Type=simple` and no `RemainAfterExit`.

systemd starts units with a minimal `PATH`. Goals needing tools outside
`/usr/bin` (cargo, uv, a user-local docker) should get a `PATH=` line via the
unit's `EnvironmentFile`, which points at `.env` in the project folder.

## Caveat for this repository's stack

`docker-compose.yml` runs `command: tcm` with `stdin_open`/`tty` — an
interactive CLI, not a daemon. Started at boot it gives you a container that is
up and waiting:

```bash
docker compose exec testcase-manager tcm
docker attach testcase-manager
```

If `tcm` exits, the container stops and nothing restarts it, while the unit is
still reported active because `up -d` succeeded. Adding `restart:
unless-stopped` to the service in `docker-compose.yml` would fix that; it
changes the stack's own definition, so it is left to you.

## Not verified here

`make service-check`, `make service-show` and `make -n service-install` were
run, and the rendered unit inspected. Nothing was installed or started: the dev
container this was written in has neither `docker` nor `systemd` (PID 1 is
`tini`). Run `make init-system` on the target host first.
