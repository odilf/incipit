# incipit

Declaratice service manager tailored for home servers.

You configure declaratively all the services you want to run from a file, and incipit handles starting the services on the corresponding ports and reverse-proxying the requests to the correct socket based on the host of the request. 

## Basic example

```toml
addr = "0.0.0.0" # Address that `incipit` binds to (usually 0.0.0.0 to expose to network)
port = 80 # Port that `incipit` listens to (80 for http and 443 for https)
incipit_host = "uoh.example.com" # Host from which to access incipit itself

# Simple service
# incipit will redirect traffic from "git.example.com" to "0.0.0.0:8264"
[[services]]
name = "git"
port = 8264
command.run = "gitea --config /path/to/app.ini"

# More elaborate service configuration
# incipit pulls the repo and runs the command automatically
[[services]]
name = "service1"
port = 6942
repo.url = "https://github.com/user/random-sveltekit-app"
repo.branch = "main"
command.build = "pnpm build"
command.run = "PORT=6942 node build"
env = { PORT = 6942 }
```

## Usage

### What is the "host"

Throughout this documentation we refer often to the host of a URL or a request. To make sure we're all in the same page, the host is what people consider the "name" of a URL. More specifically:

TODO: Put the thing here
```txt
In most cases
https://service.example.com/hello/world

In more advanced cases
https://user@password:service.example.com:2468/hello/world#fragment
```

### What about certificates?

incipit does not handle certificates at all. The recommended way to handle https and security is by using Cloudflare. The free tier is generous and you get http on their proxies without having to bother with certificates on your server. And, as a bonus, you don't expose your actual IP to the internet.







TODOs:
- [ ] Add an issue to github to add a `lazy` feature and a `autoclose` feature to be able to run, say, a minecraft server but not actually running it most of the time.
