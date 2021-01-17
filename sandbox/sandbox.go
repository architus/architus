package main;

import (
    "fmt";
    "go.starlark.net/starlark";
    // "os";
    "log";
    "strings";
    "net";
    "time";
    "math/rand";
    "math";
    "strconv";

    context "context";
    grpc "google.golang.org/grpc";
    keepalive "google.golang.org/grpc/keepalive";
    // uuid "github.com/satori/go.uuid";

    rpc "archit.us/sandbox";
)

type Sandbox struct {
    rpc.SandboxServer;
}

func (c *Sandbox) RunStarlarkScript(ctx context.Context, in *rpc.StarlarkScript) (*rpc.ScriptOutput, error) {
    // TODO(jjohnsonjj1251@gmail.com): Move user script into a main function
    const functions = `
p = print
def choice(iterable):
    n = len(iterable)
    if n == 0:
        return None
    i = randint(0, n)
    return iterable[i]

`;
    // script_uuid := uuid.NewV4();

    var script string = functions;

    script += "message = \"" + in.TriggerMessage + "\"; author = \"" + in.Author + "\"; count = " + strconv.FormatUint(in.Count, 10) + "\n";
    script += "caps = [";

    for _, c := range in.Captures {
        script += "\"" + c + "\", ";
    }

    script += "]\nargs = [";

    for _, c := range in.Arguments {
        script += "\"" + c + "\", ";
    }

    script += "]\n"
    script += "def main():\n\t"
    script += in.Script;
    script += "\nmain()";

    sin := func(thread *starlark.Thread, b *starlark.Builtin, args starlark.Tuple, kwargs []starlark.Tuple) (starlark.Value, error) {
        var rad float64 = 0.0;
        if err := starlark.UnpackArgs(b.Name(), args, kwargs, "rad", &rad); err != nil {
            return nil, err;
        }

        var s float64 = math.Sin(rad);

        return starlark.Float(s), nil;
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
        "sin": starlark.NewBuiltin("sin", sin),
    };

    var messages []string;
    thread := &starlark.Thread{
        Name: "sandbox_thread",
        Print: func(_ *starlark.Thread, msg string) { messages = append(messages, msg); },
    };

    starChan := make(chan error, 1);
    // _, runtime_err := starlark.ExecFile(thread, script_name, nil, nil);
    go func() {
        _, tmpE := starlark.ExecFile(thread, "sandbox_script.star", script, predeclared);
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
        log.Print(runtime_err.Error());
        return &rpc.ScriptOutput{
            Output: "",
            Error: runtime_err.Error(),
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
