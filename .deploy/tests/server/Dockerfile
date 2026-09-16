# A host with a real systemd, because the deploy runs against a systemd *user* manager and lingering,
# and neither can be faked convincingly: PrivateDevices, XDG_RUNTIME_DIR and `systemctl --user` all
# behave differently without an init.
#
# 22.04 on purpose: it is what production runs, and it is where the systemd version matters.
FROM ubuntu:22.04
ENV DEBIAN_FRONTEND=noninteractive
RUN apt-get update -qq && apt-get install -y -qq --no-install-recommends \
      systemd systemd-sysv dbus libpam-systemd curl netcat-openbsd sudo \
 && rm -rf /var/lib/apt/lists/*
# systemd-logind must stay enabled: lingering is what keeps the user manager alive between logins,
# and the deploy depends on it.
STOPSIGNAL SIGRTMIN+3
CMD ["/lib/systemd/systemd"]
