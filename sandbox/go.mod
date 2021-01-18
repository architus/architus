module sandbox

go 1.15

require (
	archit.us/sandbox v0.0.0
	github.com/jammiess/starlark-go v0.0.4-alpha
	github.com/satori/go.uuid v1.2.0
	google.golang.org/grpc v1.34.0
)

replace (
	archit.us/sandbox => ./rpc
	go.starlark.net => github.com/jammiess/starlark-go v0.0.4-alpha
    go.starlark.net/internal => github.com/jammiess/starlark-go/internal v0.0.4-alpha
)
