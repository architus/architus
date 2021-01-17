package main;

import (
    "fmt";
    "go.starlark.net/starlark";
    "os";
    "log";
    "strings";
    "net";
    "time";
    "math/rand";

    context "context";
    grpc "google.golang.org/grpc";
    keepalive "google.golang.org/grpc/keepalive";
    uuid "github.com/satori/go.uuid";

    rpc "archit.us/sandbox";
)

type Sandbox struct {
    rpc.SandboxServer;
}

func (c *Sandbox) RunStarlarkScript(ctx context.Context, in *rpc.StarlarkScript) (*rpc.ScriptOutput, error) {
    const functions = `
p = print
def choice(iterable):
    n = len(iterable)
    if n == 0:
        return None
    i = randint(0, n)
    return iterable[i]

`;
    script_uuid := uuid.NewV4();

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

    random := func(thread *starlark.Thread, b *starlark.Builtin, args starlark.Tuple, kwargs []starlark.Tuple) (starlark.Value, error) {
        return starlark.Float(rand.Float64()), nil;
    }

    randint := func(thread *starlark.Thread, b *starlark.Builtin, args starlark.Tuple, kwargs []starlark.Tuple) (starlark.Value, error) {
        var low int = 0;
        var high int = 1;
        if err := starlark.UnpackArgs(b.Name(), args, kwargs, "low", &low, "high", &high); err != nil {
            return nil, err;
        }

        r := rand.Float64();
        var v float64 = float64(high - low);
        v *= r;
        v += float64(low);

        return starlark.MakeInt(int(v)), nil;
    }

    predeclared := starlark.StringDict{
        "random": starlark.NewBuiltin("random", random),
        "randint": starlark.NewBuiltin("randint", randint),
    };

    var messages []string;
    thread := &starlark.Thread{
        Name: script_name,
        Print: func(_ *starlark.Thread, msg string) { messages = append(messages, msg); },
    };

    starChan := make(chan error, 1);
    // _, runtime_err := starlark.ExecFile(thread, script_name, nil, nil);
    go func() {
        _, tmpE := starlark.ExecFile(thread, script_name, functions, predeclared);
        starChan <- tmpE;
    }();

    var runtime_err error;
    select {
    case runtime_err = <- starChan:
        log.Print("Something happened");
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
        log.Print(runtime_err.(*starlark.EvalError).Backtrace());
        return &rpc.ScriptOutput{
            Output: "",
            Error: runtime_err.(*starlark.EvalError).Backtrace(),
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

    grpcServer := grpc.NewServer(
        grpc.KeepaliveParams(
            keepalive.ServerParameters{
                Time:       (time.Duration(20) * time.Second),
                Timeout:    (time.Duration(5)  * time.Second),
            },
        ),
        grpc.KeepaliveEnforcementPolicy(
            keepalive.EnforcementPolicy{
                MinTime:                (time.Duration(15) * time.Second),
                PermitWithoutStream:    true,
            },
        ),
    );
    rpc.RegisterSandboxServer(grpcServer, newServer());
    fmt.Println("Starting server");
    grpcServer.Serve(lis);
}
