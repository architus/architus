module sandbox

go 1.15

require (
	archit.us/sandbox v0.0.0
	github.com/satori/go.uuid v1.2.0
	go.starlark.net v0.0.0-20201210151846-e81fc95f7bd5
	google.golang.org/grpc v1.34.0
)

replace archit.us/sandbox => ./rpc
