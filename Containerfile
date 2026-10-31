# syntax=docker/dockerfile:1
FROM alpine:3.20

# 1. Install necessary core dependencies for networking and OMP runtime
RUN apk add --no-cache curl bash nodejs npm iproute2 python3 tzdata

ARG HOST_TZ
RUN test -f "/usr/share/zoneinfo/${HOST_TZ}" && \
    ln -s "/usr/share/zoneinfo/${HOST_TZ}" /etc/localtime

RUN addgroup -g 1001 sandboxgroup && \
    adduser -u 1001 -G sandboxgroup -h /home/sandboxuser -s /bin/bash -D sandboxuser

RUN curl -fsSL https://omp.sh/install | sh -s -- --binary \
    && mv /root/.local/bin/omp /usr/local/bin/omp \
    && chmod +x /usr/local/bin/omp

COPY files/init.sh /init.sh
RUN chmod +x /init.sh

COPY files/autologin.sh /sbin/autologin.sh
RUN chmod +x /sbin/autologin.sh

COPY out/rust/x86_64-unknown-linux-musl/release/vsock-sync /usr/bin/vsock-sync
RUN chmod +rx /usr/bin/vsock-sync

COPY files/.profile /home/sandboxuser/.profile
RUN mkdir /home/sandboxuser/.omp
COPY files/config.yml /home/sandboxuser/.omp/agent/config.yml
COPY files/models.yml.template /home/sandboxuser/.omp/agent/models.yml.template

RUN chown -R sandboxuser:sandboxgroup /home/sandboxuser

RUN mkdir -p /mnt/workspace

# 4. Enforce execution layout via the custom script target
ENTRYPOINT ["/init.sh"]
