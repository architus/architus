import sandbox_pb2_grpc
import sandbox_pb2 as message

import grpc

server = "localhost:1337"
post_script = "h={\"thing\":\"johnydumb\"}; p(post(\"https://jsonplaceholder.typicode.com/posts\", headers=h, j=h)[1])"
get_script = "h={\"thing\":\"johnydumb\"}; p(get(\"http://localhost:3232\", h))"


def test_get(stub):
    output = stub.RunStarlarkScript(
        message.StarlarkScript(
            script=get_script,
            trigger_message=message.Message(
                clean="hello",
                content="hello",
                id=12345
            ),
            message_author=message.Author(
                id=87654,
                avatar_url="",
                color="blue",
                discriminator=11,
                roles=[1],
                name="jame",
                nick="jame",
                disp_name="jame",
                permissions=200
            ),
            script_author=message.Author(
                id=87654,
                avatar_url="",
                color="blue",
                discriminator=11,
                roles=[1],
                name="jame",
                nick="jame",
                disp_name="jame",
                permissions=200
            ),
            count=100,
            captures=[],
            arguments=[],
            channel=message.Channel(
                id=400,
                name="Test channel"
            )
        ))
    if output.errno != 0:
        print(f"GET test failed with error {output.errno}:{output.error}")
    else:
        print(f"GET test succeeded with output {output.output}")


def test_post(stub):
    output = stub.RunStarlarkScript(
        message.StarlarkScript(
            script=post_script,
            trigger_message=message.Message(
                clean="hello",
                content="hello",
                id=12345
            ),
            message_author=message.Author(
                id=87654,
                avatar_url="",
                color="blue",
                discriminator=11,
                roles=[1],
                name="jame",
                nick="jame",
                disp_name="jame",
                permissions=200
            ),
            script_author=message.Author(
                id=87654,
                avatar_url="",
                color="blue",
                discriminator=11,
                roles=[1],
                name="jame",
                nick="jame",
                disp_name="jame",
                permissions=200
            ),
            count=100,
            captures=[],
            arguments=[],
            channel=message.Channel(
                id=400,
                name="Test channel"
            )
        ))
    if output.errno != 0:
        print(f"POST test failed with error {output.errno}:{output.error}")
    else:
        print(f"POST test succeeded with output {output.output}")


def main():
    with grpc.insecure_channel(server) as channel:
        stub = sandbox_pb2_grpc.SandboxStub(channel)
        # test_get(stub)
        test_post(stub)


if __name__ == "__main__":
    main()
