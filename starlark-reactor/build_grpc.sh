#!/bin/sh

protoc --go_out=./rpc --go_opt=paths=source_relative \
    --go-grpc_out=./rpc --go-grpc_opt=paths=source_relative \
    --proto_path=../lib/proto ../lib/proto/sandbox.proto

python3 -m grpc_tools.protoc -I../lib/proto --python_out=./test \
    --grpc_python_out=./test ../lib/proto/sandbox.proto
