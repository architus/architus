#!/bin/sh

protoc --go_out=./rpc --go_opt=paths=source_relative \
    --go-grpc_out=./rpc --go-grpc_opt=paths=source_relative \
    --proto_path=../lib/ipc/proto ../lib/ipc/proto/sandbox.proto

python3 -m grpc_tools.protoc -I../lib/ipc/proto --python_out=./test \
    --grpc_python_out=./test ../lib/ipc/proto/sandbox.proto
