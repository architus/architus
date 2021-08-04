#!/bin/sh

python3 -m grpc_tools.protoc -I../lib/ipc/proto --python_out=../lib/ipc --grpc_python_out=../lib/ipc \
    ../lib/ipc/proto/sandbox.proto

python3 -m grpc_tools.protoc -I../lib/ipc/proto --python_out=../lib/ipc --grpc_python_out=../lib/ipc \
    ../lib/ipc/proto/feature-gate.proto

python3 -m grpc_tools.protoc -I../lib/ipc/proto --python_out=../lib/ipc --grpc_python_out=../lib/ipc \
    ../lib/ipc/proto/manager.proto
