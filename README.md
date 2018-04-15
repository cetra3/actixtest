# Actix Test Project

This project is to test the following issue on Actix Web: https://github.com/actix/actix-web/issues/176

## Setup

You will need 2 servers: the passthrough server and the listen server.

You will also need some binary files to test:

```
dd if=/dev/zero of=./broken.bin bs=1 count=210000
dd if=/dev/zero of=./working.bin bs=1 count=2100000
```

## Backend Server

This simply accepts multipart requests and prints out chunks and headers:

```
cargo run -- -s -l 0.0.0.0:7878
```

## Passthrough server

This is the client submission server.  It takes a binary request and converts it to a multipart request:

```
cargo run -- -c "http://<backend_ip>:7878" -l 0.0.0.0:7878
```

## Client

The client connects to the passthrough server:

```
curl -X PUT --data-binary "@broken.bin" http://<passthrough_ip>:7878
```