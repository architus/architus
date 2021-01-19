package main;

import (
    "fmt";
    "go.starlark.net/starlark";
    "go.starlark.net/resolve";
    "go.starlark.net/starlarkstruct";
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
    /*
    These are some of the builtin things that are there for the user's convenience.
    Currently included in this part of the code is:
    - Choice
    - Sum
    - print alias
    */
    const functions = `
p = print
def choice(iterable):
    n = len(iterable)
    if n == 0:
        return None
    i = randint(0, n)
    return iterable[i]
def sum(iterable):
    s = 0
    for i in iterable:
        s += i
    return s

`;

    // These turn on some extra functionality within the interpreter that allow for some useful things
    // such as while loops, doing things outside of a function, and mutating global values.
    resolve.AllowRecursion = true;
    resolve.AllowNestedDef = true;
    resolve.AllowLambda = true;
    resolve.AllowSet = true;
    resolve.AllowGlobalReassign = true;

    // This is starting to set up the actual script that will be passed to the interpreter.
    var script string = functions;

    // Need to set up a global variable for this before putting it into a struct because it's a list
    // and I don't know how to make a list within a call to sprintf.
    script += "author_roles = [";
    for _, r := range in.Author.Roles {
        script += strconv.FormatUint(r, 10) + ", ";
    }
    script += "]\n";

    // Various useful structs that represent aspects of the message that triggered the autoresponse.
    // `struct` is a builtin that is defined later in the program. It comes from the starlark-go repository.
    script += fmt.Sprintf("message = struct(id=%d, content=\"%s\", clean=\"%s\"\n)",
                          in.TriggerMessage.Id, in.TriggerMessage.Content, in.TriggerMessage.Clean);
    script += fmt.Sprintf("author = struct(id=%d, avatar_url=\"%s\", color=\"%s\", discrim=%d, roles=author_roles, name=\"%s\", nick=\"%s\", disp=\"%s\")\n",
                          in.Author.Id, in.Author.AvatarUrl, in.Author.Color, in.Author.Discriminator, in.Author.Name, in.Author.Nick, in.Author.DispName);
    script += fmt.Sprintf("channel = struct(id=%d, name=\"%s\")\n",
                          in.Channel.Id, in.Channel.Name);
    script += "caps = [";
    for _, c := range in.Captures {
        script += "\"" + c + "\", ";
    }
    script += "]\nargs = [";
    for _, c := range in.Arguments {
        script += "\"" + c + "\", ";
    }
    script += "]\n"

    // The actual script is no longer put in a main function anymore because with the flags set above in `resolve`
    // we can now get the full functionality of the language outside of a function. This gives the added benefit
    // of allowing users to put newlines in their scripts and not having to do some fancy logic to account for that.
    script += in.Script;

    log.Print(script);

    // These next few functions are go defined builtins. What they do should be fairly self explanatory.
    sin := func(thread *starlark.Thread, b *starlark.Builtin, args starlark.Tuple, kwargs []starlark.Tuple) (starlark.Value, error) {
        var rad string = "0.0";
        if err := starlark.UnpackArgs(b.Name(), args, kwargs, "rad", &rad); err != nil {
            return nil, err;
        }
        if s, err := strconv.ParseFloat(rad, 64); err == nil {
            return starlark.Float(math.Sin(s)), err;
        }

        return starlark.Float(-1.0), nil;
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

    // This tells the interpreter what all of our builtins are. Struct is a starlark-go specific functionality that
    // allows for creating a struct from kwarg values.
    predeclared := starlark.StringDict{
        "random": starlark.NewBuiltin("random", random),
        "randint": starlark.NewBuiltin("randint", randint),
        "sin": starlark.NewBuiltin("sin", sin),
        "struct": starlark.NewBuiltin("struct", starlarkstruct.Make),
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
