#!/bin/sh

python3 -m grpc_tools.protoc -I../lib/proto --python_out=../lib/ipc --grpc_python_out=../lib/ipc \
    ../lib/proto/sandbox.proto

python3 -m grpc_tools.protoc -I../lib/proto --python_out=../lib/ipc --grpc_python_out=../lib/ipc \
    ../lib/proto/feature-gate.proto

python3 -m grpc_tools.protoc -I../lib/proto --python_out=../lib/ipc --grpc_python_out=../lib/ipc \
    ../lib/proto/manager.proto
