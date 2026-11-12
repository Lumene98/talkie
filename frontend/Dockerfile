FROM node:24-bookworm-slim AS builder

WORKDIR /app

RUN apt-get update && \
    apt-get install -y --no-install-recommends ca-certificates && \
    rm -rf /var/lib/apt/lists/*
ENV NODE_USE_SYSTEM_CA=1
RUN npm install --global pnpm@10.30.3

COPY package.json pnpm-lock.yaml ./

RUN pnpm install --frozen-lockfile

COPY . .

RUN pnpm build

FROM nginx/nginx-unprivileged:alpine

COPY --from=builder \
    /app/dist \
    /usr/share/nginx/html

COPY nginx.conf /etc/nginx/conf.d/default.conf

EXPOSE 5173

