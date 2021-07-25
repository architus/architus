package main;

import (
    "fmt";
    "go.starlark.net/starlark";
    "go.starlark.net/resolve";
    "go.starlark.net/starlarkstruct";
    "log";
    "strings";
    "net";
    "time";
    "math/rand";
    "math";
    "strconv";
    "encoding/json";
    "net/http";
    "net/url";
    "io";
    "io/ioutil";
    "bytes";
    "os";

    context "context";
    grpc "google.golang.org/grpc";
    keepalive "google.golang.org/grpc/keepalive";

    rpc "archit.us/sandbox";
)

const MaxMessageSize = 1000000;

type Sandbox struct {
    rpc.SandboxServer;
}

type Author struct {
    Id uint64
    Name string
    AvatarUrl string
    Color string
    Discrim uint32
    Roles []uint64
    Nick string
    Display_name string
    Permissions uint64
}

func sin(thread *starlark.Thread, b *starlark.Builtin, args starlark.Tuple, kwargs []starlark.Tuple) (starlark.Value, error) {
    var rad float64 = 0.0;
    if err := starlark.UnpackArgs(b.Name(), args, kwargs, "rad", &rad); err != nil {
        return nil, err;
    }

    return starlark.Float(math.Sin(rad)), nil;
}

func random(thread *starlark.Thread, b *starlark.Builtin, args starlark.Tuple, kwargs []starlark.Tuple) (starlark.Value, error) {
    return starlark.Float(rand.Float64()), nil;
}

func randint(thread *starlark.Thread, b *starlark.Builtin, args starlark.Tuple, kwargs []starlark.Tuple) (starlark.Value, error) {
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

func (c *Sandbox) RunStarlarkScript(ctx context.Context, in *rpc.StarlarkScript) (*rpc.ScriptOutput, error) {
    /*
    These are some of the builtin things that are there for the user's convenience.
    Currently included in this part of the code is:
    - Choice
    - Sum
    - print alias
    */
    const functions = `
load("json.star", "json")
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
def post(url, headers=None, data=None, j=None):
    if j != None:
        j = json.encode(j)
    (resp_code, body) = post_internal(url, headers, data, j)
    json_values = json.decode(body)
    return (resp_code, json_values)
    #return (resp_code, body)

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
    for _, r := range in.MessageAuthor.Roles {
        script += strconv.FormatUint(r, 10) + ", ";
    }
    script += "]\n";

    // Various useful structs that represent aspects of the message that triggered the autoresponse.
    // `struct` is a builtin that is defined later in the program. It comes from the starlark-go repository.
    // For all strings, the variables need to be made in go and then passed into the interpreter so that
    // random special characters don't break everything.
    script += fmt.Sprintf("message = struct(id=%d, content=message_content_full, clean=message_clean_full)\n",
                          in.TriggerMessage.Id);
    script += fmt.Sprintf("author = struct(id=%d, avatar_url=\"%s\", color=\"%s\", discrim=%d, roles=author_roles, name=\"%s\", nick=\"%s\", disp=\"%s\", perms=%d)\n",
                          in.MessageAuthor.Id, in.MessageAuthor.AvatarUrl, in.MessageAuthor.Color, in.MessageAuthor.Discriminator,
                          in.MessageAuthor.Name, in.MessageAuthor.Nick, in.MessageAuthor.DispName, in.MessageAuthor.Permissions);
    script += fmt.Sprintf("channel = struct(id=%d, name=channel_name)\n",
                          in.Channel.Id);
    script += fmt.Sprintf("count = %d\n", in.Count);
    script += "msg = message; a = author; ch = channel;\n";

    var author = make([]starlark.Value, 3);
    author[0] = starlark.String(in.MessageAuthor.Name);
    author[1] = starlark.String(in.MessageAuthor.Nick);
    author[2] = starlark.String(in.MessageAuthor.DispName);

    channel_name := starlark.String(in.Channel.Name);

    var caps = make([]starlark.Value, len(in.Captures));
    for i, c := range in.Captures {
        caps[i] = starlark.String(c);
    }

    var args = make([]starlark.Value, len(in.Arguments));
    for i, c := range in.Arguments {
        args[i] = starlark.String(c);
    }

    // The actual script is no longer put in a main function anymore because with the flags set above in `resolve`
    // we can now get the full functionality of the language outside of a function. This gives the added benefit
    // of allowing users to put newlines in their scripts and not having to do some fancy logic to account for that.
    script += in.Script;

    // These next few functions are go defined builtins. What they do should be fairly self explanatory.

    get := func(thread *starlark.Thread, b *starlark.Builtin, args starlark.Tuple, kwargs []starlark.Tuple) (starlark.Value, error) {
        var raw_url string = "";
        var headers map[string]string = nil;
        if err := starlark.UnpackArgs(b.Name(), args, kwargs, "url", &raw_url, "?headers", &headers); err != nil {
            return nil, err;
        }

        get_url, err := url.Parse(raw_url);
        if err != nil {
            return nil, err;
        }

        var get_header http.Header;
        if (headers != nil) {
            for k, v := range headers {
                get_header.Set(k, v);
            }
        }

        mauthor := Author{
            Name: in.MessageAuthor.Name,
            Id: in.MessageAuthor.Id,
            AvatarUrl: in.MessageAuthor.AvatarUrl,
            Color: in.MessageAuthor.Color,
            Discrim: in.MessageAuthor.Discriminator,
            Roles: in.MessageAuthor.Roles,
            Nick: in.MessageAuthor.Nick,
            Permissions: in.MessageAuthor.Permissions,
        }

        sauthor := Author{
            Name: in.ScriptAuthor.Name,
            Id: in.ScriptAuthor.Id,
            AvatarUrl: in.ScriptAuthor.AvatarUrl,
            Color: in.ScriptAuthor.Color,
            Discrim: in.ScriptAuthor.Discriminator,
            Roles: in.ScriptAuthor.Roles,
            Nick: in.ScriptAuthor.Nick,
            Permissions: in.ScriptAuthor.Permissions,
        }

        mauthor_json, err := json.Marshal(mauthor);
        if err != nil {
            return nil, err;
        }

        sauthor_json, err := json.Marshal(sauthor);
        if err != nil {
            return nil, err;
        }

        get_header.Set("X-Arch-Author", string(mauthor_json));
        get_header.Set("X-Arch-Script-Author", string(sauthor_json));
        get_header.Set("User-Agent", "Mozilla/5.0 (compatible; Architus/1.0; +https://archit.us");

        var req http.Request;
        req.Method = http.MethodGet;
        req.URL = get_url;
        req.Header = get_header;

        var client http.Client;
        resp, err := client.Do(&req);
        if err != nil {
            return nil, err;
        }

        limited_body := io.LimitReader(resp.Body, MaxMessageSize);
        bytes, err := io.ReadAll(limited_body);
        if err != nil {
            return nil, err;
        }
        resp.Body.Close();

        resp_code := starlark.MakeInt(resp.StatusCode);
        body := starlark.String(bytes);
        tup := make([]starlark.Value, 2);
        tup[0] = resp_code;
        tup[1] = body;
        return starlark.Tuple(tup), nil;
    }

    post_internal := func(thread *starlark.Thread, b *starlark.Builtin, args starlark.Tuple, kwargs []starlark.Tuple) (starlark.Value, error) {
        var raw_url string = "";
        var headers map[string]string = nil;
        var data []byte = nil;
        var user_json string = "";
        if err := starlark.UnpackArgs(b.Name(), args, kwargs, "url", &raw_url, "?headers", &headers, "?data", &data, "?json", &user_json); err != nil {
            return nil, err;
        }

        post_url, err := url.Parse(raw_url);
        if err != nil {
            return nil, err;
        }

        var post_header http.Header;
        if (headers != nil) {
            for k, v := range headers {
                post_header.Set(k, v);
            }
        }

        mauthor := Author{
            Name: in.MessageAuthor.Name,
            Id: in.MessageAuthor.Id,
            AvatarUrl: in.MessageAuthor.AvatarUrl,
            Color: in.MessageAuthor.Color,
            Discrim: in.MessageAuthor.Discriminator,
            Roles: in.MessageAuthor.Roles,
            Nick: in.MessageAuthor.Nick,
            Permissions: in.MessageAuthor.Permissions,
        }

        sauthor := Author{
            Name: in.ScriptAuthor.Name,
            Id: in.ScriptAuthor.Id,
            AvatarUrl: in.ScriptAuthor.AvatarUrl,
            Color: in.ScriptAuthor.Color,
            Discrim: in.ScriptAuthor.Discriminator,
            Roles: in.ScriptAuthor.Roles,
            Nick: in.ScriptAuthor.Nick,
            Permissions: in.ScriptAuthor.Permissions,
        }

        mauthor_json, err := json.Marshal(mauthor);
        if err != nil {
            return nil, err;
        }

        sauthor_json, err := json.Marshal(sauthor);
        if err != nil {
            return nil, err;
        }
        post_header.Set("X-Arch-Author", string(mauthor_json));
        post_header.Set("X-Arch-Script-Author", string(sauthor_json));
        post_header.Set("User-Agent", "Mozilla/5.0 (compatible; Architus/1.0; +https://archit.us");

        var req http.Request;
        req.Method = http.MethodPost;
        req.URL = post_url;
        req.Header = post_header;
        if (user_json != "") {
            req.Body = ioutil.NopCloser(strings.NewReader(user_json));
        } else if (data != nil) {
            req.Body = ioutil.NopCloser(bytes.NewReader(data));
        } else {
            req.Body = nil;
        }

        var client http.Client;
        resp, err := client.Do(&req);
        if err != nil {
            return nil, err;
        }

        limited_body := io.LimitReader(resp.Body, MaxMessageSize);
        bytes, err := io.ReadAll(limited_body);
        if err != nil {
            return nil, err;
        }
        resp.Body.Close();

        resp_code := starlark.MakeInt(resp.StatusCode);
        body := starlark.String(bytes);
        tup := make([]starlark.Value, 2);
        tup[0] = resp_code;
        tup[1] = body;
        return starlark.Tuple(tup), nil;
    }

    // This tells the interpreter what all of our builtins are. Struct is a starlark-go specific functionality that
    // allows for creating a struct from kwarg values.
    // TODO(jjohnson): Add get and post builtins
    predeclared := starlark.StringDict{
        "random": starlark.NewBuiltin("random", random),
        "randint": starlark.NewBuiltin("randint", randint),
        "sin": starlark.NewBuiltin("sin", sin),
        "struct": starlark.NewBuiltin("struct", starlarkstruct.Make),
        "post_internal": starlark.NewBuiltin("post_internal", post_internal),
        "get": starlark.NewBuiltin("get", get),
        "message_content_full": starlark.String(in.TriggerMessage.Content),
        "message_clean_full": starlark.String(in.TriggerMessage.Clean),
        "caps": starlark.NewList(caps),
        "args": starlark.NewList(args),
        "auth_list": starlark.NewList(author),
        "channel_name": channel_name,
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
        log.Print(runtime_err);
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
    proxy := os.Getenv("HTTP_PROXY");
    if proxy != "" {
        fmt.Print("Starting production server with proxy: ");
        fmt.Println(proxy);
    } else {
        fmt.Println("Starting debug server without a proxy");
    }
    grpcServer.Serve(lis);
}
