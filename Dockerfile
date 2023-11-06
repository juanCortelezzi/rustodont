FROM rust:1.71 as builder

RUN USER=root cargo new --bin rustodont
WORKDIR /rustodont
COPY ./Cargo.toml .
COPY ./Cargo.lock .
RUN cargo build --release
RUN rm -rf src
RUN rm ./target/release/deps/rustodont*

RUN cargo install sqlx-cli

COPY ./src ./src
COPY ./migrations ./migrations

RUN touch sqeel_test.db
# RUN sqlx database setup -D sqlite://sqeel_test.db
RUN cargo sqlx prepare -D sqlite://sqeel_test.db
RUN cargo build --release

FROM debian:bullseye-slim
ARG APP=/usr/src/app

RUN apt-get update \
    && apt-get install -y ca-certificates tzdata \
    && rm -rf /var/lib/apt/lists/*


ENV TZ=Etc/UTC \
    APP_USER=appuser

RUN groupadd $APP_USER \
    && useradd -g $APP_USER $APP_USER \
    && mkdir -p ${APP}

COPY --from=builder /rustodont/target/release/rustodont ${APP}/rustodont

RUN chown -R $APP_USER:$APP_USER ${APP}

USER $APP_USER
WORKDIR ${APP}

COPY ./.env .

EXPOSE 3000

CMD ["./rustodont"]
