#!/bin/sh
# isolate --cg needs a cgroup v2 subtree delegated to isolate-cg-keeper.
# The keeper can only enable controllers on a cgroup that has no processes
# directly in it (cgroup v2's "no internal process" rule), so before
# starting it we move this process into a leaf subgroup, then start the
# keeper directly in the now-empty root cgroup.
# Best-effort: without a Linux host or privileged/cgroupns=host access this
# fails, in which case we log and start the API anyway (health checks and
# task management still work, only judging via isolate --cg would fail).
setup_isolate_cgroup() {
  cg_rel=$(sed -n 's/^0::\(.*\)$/\1/p' /proc/self/cgroup)
  [ -n "$cg_rel" ] || return 1
  cg="/sys/fs/cgroup$cg_rel"
  [ -w "$cg/cgroup.procs" ] || return 1

  mkdir -p "$cg/init"
  echo $$ >"$cg/init/cgroup.procs"

  # A `( )` subshell keeps the parent's $$ in both dash and bash, so it
  # can't move itself by PID - use a real child process via `sh -c` instead.
  sh -c "echo \$\$ >'$cg/cgroup.procs' && exec /usr/local/sbin/isolate-cg-keeper" &
}

setup_isolate_cgroup ||
  echo "WARN: isolate cgroup delegation not set up, isolate --cg will fail" >&2

exec "$@"
