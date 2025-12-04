# Banette

[![License: MPL 2.0](https://img.shields.io/badge/License-MPL%202.0-brightgreen.svg)](https://opensource.org/licenses/MPL-2.0)

**Banette** is an Unreal Engine plugin that provides a modular, layer-based architecture for building network clients with support for coroutines. Inspired by the "[tower](https://github.com/tower-rs/tower)" service pattern, it enables clean composition of cross-cutting concerns like retries, rate limiting, header injection, and JSON parsing.

## Features

- **Layer-based Architecture**: Compose services by stacking layers (middleware), each handling a specific concern
- **Coroutine Support**: Built on [UE5Coro](https://github.com/landelare/ue5coro) for async/await style code
- **HTTP Transport**: Native HTTP client implementation using Unreal's HTTP module
- **Reusable Layers**: Pre-built layers for common patterns (retry, rate limiting, header injection, JSON parsing, URL origin)
- **OpenAPI Code Generation**: Rust-based generator that creates Unreal Engine C++ code from OpenAPI specifications
- **Blueprint Integration**: Expose generated APIs as Blueprint-callable functions
- **Unified Error Handling**: Structured error types using Unreal's experimental UnifiedError system

## Project Status

⚠️ **Beta** – This plugin is marked as beta (`IsBetaVersion: true`). The API may change between versions.

## Prerequisites

- **Unreal Engine 5.x** (compatible with UE5's module system)
- **[UE5Coro Plugin](https://github.com/landelare/ue5coro)** – Required dependency for coroutine support
- **Rust Toolchain** (for the code generator):
  - Rust stable (`x86_64-pc-windows-msvc` on Windows, `x86_64-unknown-linux-gnu` on Linux)
  - Visual Studio 2022 with C++ build tools (Windows)
  - Note: The automatic pre-build integration is Windows-only; Linux requires manual generator builds

## Installation

1. git submodule this repository into your project's `Plugins/` directory:
   ```
   YourProject/
   └── Plugins/
       └── Banette/
   ```

2. Ensure the [UE5Coro](https://github.com/landelare/ue5coro) plugin is also installed in your project.

3. Regenerate project files and build.

## Module Overview

Banette is organized into five modules, each with specific responsibilities:

### Banette (Core)

The foundational module providing the core abstractions for the service/layer pattern.

| Component | Description |
|-----------|-------------|
| `TService<Request, Response>` | Base template for services that process requests and return responses asynchronously |
| `TLayer<InService, OutService>` | Base template for layers that wrap services to add behavior |
| `TResult<Value, Error>` | Result type combining value-or-error semantics with UnifiedError |
| `TServiceBuilder<>` | Fluent builder for composing services with multiple layers |
| `TServiceProvider<>` | Singleton pattern for service instance management |


### BanetteTransport

Provides concrete HTTP client implementation built on Unreal's HTTP module.

| Component | Description |
|-----------|-------------|
| `FHttpService` | HTTP client implementation with coroutine-based async calls |
| `FHttpRequest` | Request data structure with builder pattern for configuration |
| `FHttpResponse` | Response data structure with status, headers, and body |
| `EHttpMethod` | Enum for HTTP methods (GET, POST, PUT, DELETE, PATCH, HEAD) |

**Error Codes:**
- `InvalidUrl` – Empty or invalid URL
- `RequestCreationFailed` – Failed to create HTTP request
- `ConnectionFailed` – HTTP connection failure
- `NoResponse` – No response received

### BanetteKit

A collection of ready-to-use layers for common HTTP patterns:

| Layer | Description |
|-------|-------------|
| `FHttpOriginLayer` | Prefixes relative URLs with a base origin (e.g., `https://api.example.com`) |
| `FInjectHeaderLayer` | Injects headers into every request (e.g., authentication tokens) |
| `FJsonLayer` | Parses HTTP response bodies as JSON, producing `FHttpJsonResponse` |
| `TRetryLayer<Service>` | Retries failed requests with configurable attempts and delays |
| `TRateLimitLayer<Service>` | Token bucket rate limiting with optional async waiting |

### BanetteGenerator

A Rust-based code generator that creates Unreal Engine C++ code from OpenAPI 3.x specifications.

| Component | Description |
|-----------|-------------|
| Rust CLI (`generator`) | Command-line tool for code generation |
| Blueprint Function | `UBanetteGeneratorLibrary::GenerateOpenApi()` for editor integration |
| Tera Templates | Template files for C++ code generation |

**Supported Features:**
- OpenAPI 3.x (JSON and YAML)
- Local files and HTTP URLs
- Schema-to-USTRUCT conversion
- Path-to-function mapping
- Request/response body handling

### BanetteTest

Testing utilities and example code demonstrating plugin usage.

## Usage Examples

### Basic HTTP Request

```cpp
#include "Banette.h"
#include "BanetteTransport/Http/HttpService.h"

using namespace Banette::Transport::Http;
using namespace Banette::Core;

// Inside a coroutine context
UE5Coro::TCoroutine<> MakeRequest()
{
    auto HttpService = MakeShared<FHttpService>();

    FHttpRequest Request;
    Request.Url = TEXT("https://api.example.com/data");
    Request.Method = EHttpMethod::Get;

    TResult<FHttpResponse> Result = co_await HttpService->Call(Request);

    if (Result.IsValid())
    {
        const FHttpResponse& Response = Result.GetValue();
        UE_LOG(LogTemp, Log, TEXT("Status: %d"), Response.StatusCode);
    }
    else
    {
        UE_LOG(LogTemp, Error, TEXT("Request failed"));
    }
}
```

### Composing Layers with ServiceBuilder

```cpp
#include "Banette.h"
#include "BanetteTransport/Http/HttpService.h"
#include "BanetteKit/Layers/HttpOriginLayer.h"
#include "BanetteKit/Layers/RetryLayer.h"
#include "BanetteKit/Layers/InjectHeaderLayer.h"

using namespace Banette::Transport::Http;
using namespace Banette::Pipeline;
using namespace Banette::Kit;

// Create a fully configured HTTP service
TSharedRef<FHttpService> CreateApiService()
{
    auto BaseService = MakeShared<FHttpService>();

    // Configure layers
    FHttpOriginLayer OriginLayer(TEXT("https://api.example.com"));
    
    FInjectHeaderLayer AuthLayer;
    AuthLayer.LazyHeader(TEXT("Authorization"), []()-> UE5Coro::TCoroutine<FString>
       {
           // Simulate async token retrieval
           co_await UE5Coro::AsyncDelay(0.1f);
           FString JsonWebToken = TEXT("your-jwt-token");
           co_return FString::Format(TEXT("Bearer {0}"), {JsonWebToken});
       }
    );
    
    TRetryLayer<FHttpService>::FRetryConfig RetryConfig;
    RetryConfig.MaxAttempts = 3;
    RetryConfig.DelayBetweenRetries = 0.5f;
    TRetryLayer<FHttpService> RetryLayer(RetryConfig);

    // Compose service with layers
    return TServiceBuilder<>::New(BaseService)
        .Layer(OriginLayer)     // Add base URL
        .Layer(AuthLayer)       // Add auth header
        .Layer(RetryLayer)      // Add retry logic
        .Build();
}
```

### Using Request Builder Pattern

```cpp
FHttpRequest Request = FHttpRequest()
    .With_Url(TEXT("/users/123"))
    .With_Method(EHttpMethod::Get)
    .With_TimeoutSeconds(30.0f)
    .AddHeader(TEXT("Accept"), TEXT("application/json"));
```

### JSON Response Handling

```cpp
#include "BanetteKit/Layers/JsonLayer.h"

using namespace Banette::Kit;

// Create JSON-enabled service
auto HttpService = MakeShared<FHttpService>();
FJsonLayer JsonLayer;

auto JsonService = TServiceBuilder<>::New(HttpService)
    .Layer(JsonLayer)
    .Build();

// Response will have parsed JSON
TResult<FHttpJsonResponse> Result = co_await JsonService->Call(Request);

if (Result.IsValid())
{
    const FHttpJsonResponse& Response = Result.GetValue();
    
    // Parse into a USTRUCT
    FMyDataStruct Data;
    if (Response.GetContent(Data))
    {
        // Use parsed data
    }
}
```

### Rate Limiting

```cpp
#include "BanetteKit/Layers/RateLimitLayer.h"

TRateLimitLayer<FHttpService>::FRateLimitConfig Config;
Config.TokensPerSecond = 5.0;  // 5 requests per second
Config.MaxTokens = 10.0;       // Burst capacity
Config.bWaitForToken = true;   // Wait if rate limited
TRateLimitLayer<FHttpService> RateLimitLayer(Config);
```

### Using TServiceProvider with Generator

`TServiceProvider` enables singleton-style service management and integrates with the code generator. Implement the `buildService` function to configure the HTTP client used for API calls:

```cpp
/// You propbably want to put this file in generator's extra-headers parameter

using FAnxHttpApiService = TService<FHttpRequest, FHttpJsonResponse>;

template <>
struct Banette::Pipeline::TServiceProvider<FAnxHttpApiService>
{
	BANETTE_SERVICE_PROVIDER(FAnxHttpApiService)
	{
		const auto HttpService = MakeShared<FHttpService>();

		FInjectHeaderLayer InjectHeader{};
		const TRetryLayer<FHttpService> Retry({});
		const TRateLimitLayer<FHttpService> RateLimit({});
		const FJsonLayer Json{};
		FHttpOriginLayer OriginLayer(TEXT("http://127.0.0.1:10802"));

		InjectHeader.
			AddHeader(TEXT("Authorization"), FString::Format(TEXT("Bearer {0}"), {"debug-token"}));


		return TServiceBuilder<>::New(HttpService)
		       .Layer(OriginLayer)
		       .Layer(InjectHeader)
		       .Layer(RateLimit)
		       .Layer(Retry)
		       .Layer(Json)
		       .Build();
	}
};
```

## OpenAPI Code Generation

### Using the Command Line

```bash
cd Source/BanetteGenerator/generator
cargo build --release

./target/release/generator \
    --path https://api.example.com/openapi.json \
    --output-dir ../Generated \
    --file-name MyApi.h \
    --module-name MYMODULE_API \
    --extra-headers "MyTypes.h;MyUtils.h"
```

### Using Blueprint

The `UBanetteGeneratorLibrary::GenerateOpenApi` function is exposed to Blueprints:

```cpp
UBanetteGeneratorLibrary::GenerateOpenApi(
    TEXT("https://api.example.com/openapi.yaml"),
    TEXT("C:/Project/Source/Generated"),
    TEXT("MyApi.h"),
    TEXT("MYMODULE_API"),
    TEXT("CustomTypes.h")
);
```

### Generated Code Structure

The generator produces:
- USTRUCTs for each schema in the OpenAPI spec
- A `UBlueprintFunctionLibrary` with latent Blueprint functions for each endpoint
- Proper Unreal type mappings (FString, int32, TArray, etc.)

## Building

### Unreal Engine Plugin

The plugin builds automatically with your Unreal project. Ensure you have:

1. UE5Coro plugin installed
2. C++ project (not Blueprint-only)

### Rust Generator

```bash
# Windows: Set Visual Studio path (if not using the default location)
# The PreBuildSteps.bat uses VSROOT to find vcvars64.bat
set VSROOT=C:\Program Files\Microsoft Visual Studio\2022\Community

# Navigate to the generator directory
cd Source/BanetteGenerator/generator

# Build the generator (Windows)
cargo build --release --target x86_64-pc-windows-msvc

# Build the generator (Linux)
cargo build --release --target x86_64-unknown-linux-gnu
```

The pre-build step in `Banette.uplugin` automatically builds the Rust generator on Windows.

## Running Tests

The `BanetteTest` module contains test utilities. Tests can be run through Unreal's automation system or by calling the test functions directly in the editor.

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     Your Application                        │
├─────────────────────────────────────────────────────────────┤
│                    Generated API Layer                      │
│              (from OpenAPI via BanetteGenerator)            │
├─────────────────────────────────────────────────────────────┤
│                      BanetteKit                             │
│    ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────────────┐  │
│    │ Retry   │ │ Rate    │ │ Origin  │ │ Header Injection│  │
│    │ Layer   │ │ Limit   │ │ Layer   │ │ Layer           │  │
│    └────┬────┘ └────┬────┘ └────┬────┘ └────────┬────────┘  │
│         │           │           │               │           │
│         └───────────┴───────────┴───────────────┘           │
│                           │                                 │
├───────────────────────────┼─────────────────────────────────┤
│                   BanetteTransport                          │
│              ┌────────────────────────────────┐             │
│              │       FHttpService             │             │
│              │      (HTTP Client)             │             │
│              └────────────────────────────────┘             │
│              ┌────────────────────────────────┐             │
│              │     Custom Transport           │             │
│              │       (Extensible)             │             │
│              └────────────────────────────────┘             │
├─────────────────────────────────────────────────────────────┤
│                      Banette (Core)                         │
│         TService, TLayer, TResult, ServiceBuilder           │
├─────────────────────────────────────────────────────────────┤
│                      UE5Coro                                │
│              (Coroutine Infrastructure)                     │
└─────────────────────────────────────────────────────────────┘
```

## Contributing

Contributions are welcome! Please ensure your code follows Unreal Engine coding standards.

## License

This project is licensed under the **Mozilla Public License 2.0** (MPL-2.0). See the [LICENSE](LICENSE) file for details.

## Author

Created by [tarnishablec](mailto:tarnishablec@outlook.com)
