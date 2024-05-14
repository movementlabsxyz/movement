FROM alpine:3.12

RUN apk add --no-cache git openssh-client bash curl jq

# Install GitHub CLI
RUN curl -L https://github.com/cli/cli/releases/download/v1.9.2/gh_1.9.2_linux_amd64.tar.gz | tar xz && \
    mv gh_1.9.2_linux_amd64/bin/gh /usr/bin/gh

COPY entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh

ENTRYPOINT ["/entrypoint.sh"]
