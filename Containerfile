# syntax=docker/dockerfile:1
FROM alpine:3.20

# 1. Install necessary core dependencies for networking and OMP runtime
RUN apk add --no-cache curl bash nodejs npm iproute2

RUN addgroup -g 1001 sandboxgroup && \
    adduser -u 1001 -G sandboxgroup -h /home/sandboxuser -s /bin/bash -D sandboxuser

RUN curl -fsSL https://omp.sh/install | sh -s -- --binary \
    && mv /root/.local/bin/omp /usr/local/bin/omp \
    && chmod +x /usr/local/bin/omp

COPY files/init.sh /init.sh
RUN chmod +x /init.sh
COPY files/autologin.sh /sbin/autologin.sh
RUN chmod +x /sbin/autologin.sh
COPY files/.profile /home/sandboxuser/.profile
RUN chown sandboxuser:sandboxgroup /home/sandboxuser/.profile

RUN mkdir -p /mnt/repos

# 4. Enforce execution layout via the custom script target
ENTRYPOINT ["/init.sh"]
