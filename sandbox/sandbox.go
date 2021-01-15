package main;

import (
    "fmt";
    "go.starlark.net/starlark";
    "os";
    "log";
    "strings";
    "net";
    "time";

    context "context";
    grpc "google.golang.org/grpc";
    uuid "github.com/satori/go.uuid";

    rpc "archit.us/sandbox";
)

type Sandbox struct {
    rpc.SandboxServer;
}

func (c *Sandbox) RunStarlarkScript(ctx context.Context, in *rpc.StarlarkScript) (*rpc.ScriptOutput, error) {
    script_uuid := uuid.NewV4();

    /*
    if err != nil {
        log.Print("Failed to generate filename");
        return rpc.ScriptOutput{
            Output: "",
            Error: "Failed to generate unique filename",
            Errno: 1,
        }, nil;
    }
    */

    script_name := script_uuid.String();
    f, file_err := os.Create(script_name);

    if file_err != nil {
        log.Print("Failed to create temporary file for script");
        return &rpc.ScriptOutput{
            Output: "",
            Error: "Failed to create temporary file for script",
            Errno: 2,
        }, nil;
    }

    // Need to be in this order because defer adds to a stack => LIFO
    defer os.Remove(script_name);
    defer f.Close();

    _, w1err := fmt.Fprintf(f, "message = \"%s\"; author = \"%s\"; count = %d;\n", in.TriggerMessage, in.Author, in.Count);

    if w1err != nil {
        log.Print("Failed to write to script file");
        return &rpc.ScriptOutput{
            Output: "",
            Error: "Failed to write to script file",
            Errno: 3,
        }, nil;
    }

    _, w2err := fmt.Fprintf(f, "caps = [");

    if w2err != nil {
        log.Print("Failed to write to script file");
        return &rpc.ScriptOutput{
            Output: "",
            Error: "Failed to write to script file",
            Errno: 3,
        }, nil;
    }

    for _, c := range in.Captures {
        _, w3err := fmt.Fprintf(f, "\"%s\", ", c);
        if w3err != nil {
            log.Print("Failed to write to script file");
            return &rpc.ScriptOutput{
                Output: "",
                Error: "Failed to write to script file",
                Errno: 3,
            }, nil;
        }
    }

    _, w4err := fmt.Fprintf(f, "]\nargs = [");

    if w4err != nil {
        log.Print("Failed to write to script file");
        return &rpc.ScriptOutput{
            Output: "",
            Error: "Failed to write to script file",
            Errno: 3,
        }, nil;
    }

    for _, c := range in.Arguments {
        _, w5err := fmt.Fprintf(f, "\"%s\", ", c);
        if w5err != nil {
            log.Print("Failed to write to script file");
            return &rpc.ScriptOutput{
                Output: "",
                Error: "Failed to write to script file",
                Errno: 3,
            }, nil;
        }
    }

    _, w6err := fmt.Fprintf(f, "]\n");

    if w6err != nil {
        log.Print("Failed to write to script file");
        return &rpc.ScriptOutput{
            Output: "",
            Error: "Failed to write to script file",
            Errno: 3,
        }, nil;
    }

    _, w7err := fmt.Fprintf(f, in.Script);

    if w7err != nil {
        log.Print("Failed to write to script file");
        return &rpc.ScriptOutput{
            Output: "",
            Error: "Failed to write to script file",
            Errno: 3,
        }, nil;
    }

    var messages []string;
    thread := &starlark.Thread{
        Name: script_name,
        Print: func(_ *starlark.Thread, msg string) { messages = append(messages, msg); },
    };

    starChan := make(chan error, 1);
    // _, runtime_err := starlark.ExecFile(thread, script_name, nil, nil);
    go func() {
        _, tmpE := starlark.ExecFile(thread, script_name, nil, nil);
        starChan <- tmpE;
    }();

    var runtime_err error;
    select {
    case runtime_err = <- starChan:
        log.Print("Successfully completed a script");
    case <- time.After(time.Second):
        log.Print("Script timed out");
        return &rpc.ScriptOutput{
            Output: "",
            Error: "Script timed out",
            Errno: 5,
        }, nil;
    }

    if runtime_err != nil {
        log.Print("Script failed to run");
        return &rpc.ScriptOutput{
            Output: "",
            Error: "Script failed to run",
            Errno: 4,
        }, nil;
    }

    return &rpc.ScriptOutput{
        Output: strings.Join(messages, "\n"),
        Error: "",
        Errno: 0,
    }, nil;
}

func newServer() *Sandbox {
    return &Sandbox{};
}

func main() {
    lis, sock_err := net.Listen("tcp", "0.0.0.0:1337");
    if sock_err != nil {
        log.Fatal("Failed to connect to socket");
    }

    grpcServer := grpc.NewServer();
    rpc.RegisterSandboxServer(grpcServer, newServer());
    fmt.Println("Starting server");
    grpcServer.Serve(lis);
}
