# A real HAProxy, because the rollout reads `show stat` over the runtime socket and the field layout,
# the transitional `UP 1/100` status and the `bck` flag are all things a mock would get wrong.
#
# 2.4 matches production. The tools are baked in rather than installed at run time, so a suite cannot
# pass or fail depending on whether apt-get worked that day.
FROM haproxy:2.4
USER root
ENV DEBIAN_FRONTEND=noninteractive
RUN apt-get update -qq \
 && apt-get install -y -qq --no-install-recommends socat curl procps netcat-openbsd \
      openssh-client sudo iproute2 \
 && rm -rf /var/lib/apt/lists/* \
 && command -v socat && command -v curl && command -v pgrep && command -v scp && command -v sudo
