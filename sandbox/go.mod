module sandbox

go 1.15

require (
	archit.us/sandbox v0.0.0
	go.starlark.net v0.0.0-20210119162422-73f535f109ef // indirect
	google.golang.org/grpc v1.34.0
)

replace archit.us/sandbox => ./rpc
