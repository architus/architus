#!/bin/sh

python3 -m grpc_tools.protoc -I../lib/proto --python_out=../lib/python-common/ipc  --grpc_python_out=../lib/python-common/ipc  \
    ../lib/proto/sandbox.proto

python3 -m grpc_tools.protoc -I../lib/proto --python_out=../lib/python-common/ipc  --grpc_python_out=../lib/python-common/ipc \
    ../lib/proto/feature-gate.proto

python3 -m grpc_tools.protoc -I../lib/proto --python_out=../lib/python-common/ipc  --grpc_python_out=../lib/python-common/ipc \
    ../lib/proto/manager.proto
