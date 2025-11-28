// Copyright 2019-Present tarnishablec. All Rights Reserved.

#pragma once
#include "Banette.h"
#include "Experimental/UnifiedError/UnifiedError.h"
#include "Containers/Map.h"
#include "Containers/Array.h"

// UnifiedError declarations for HTTP transport failures
// Expose stable error codes so callers can branch on explicit conditions.
UE_DECLARE_ERROR_MODULE(BANETTETRANSPORT_API, Banette::Transport::Http);

UE_DECLARE_ERROR(BANETTETRANSPORT_API, InvalidUrl, 1, Banette::Transport::Http,
                 NSLOCTEXT("BanetteHttp", "InvalidUrl", "Invalid or empty URL."));

UE_DECLARE_ERROR(BANETTETRANSPORT_API, RequestCreationFailed, 2, Banette::Transport::Http,
                 NSLOCTEXT("BanetteHttp", "RequestCreationFailed", "Failed to create HTTP request."));

UE_DECLARE_ERROR(BANETTETRANSPORT_API, ConnectionFailed, 3, Banette::Transport::Http,
                 NSLOCTEXT("BanetteHttp", "ConnectionFailed", "HTTP connection failed."));

UE_DECLARE_ERROR(BANETTETRANSPORT_API, NoResponse, 4, Banette::Transport::Http,
                 NSLOCTEXT("BanetteHttp", "NoResponse", "No HTTP response received."));


namespace Banette::Transport::Http
{
	using namespace Banette::Core;

	// Supported HTTP methods for the transport layer.
	enum class EHttpMethod : uint8
	{
		Get,
		Post,
		Put,
		Delete,
		Patch,
		Head
	};

	// Request data for HTTP calls.
	struct BANETTETRANSPORT_API FHttpRequest
	{
		// Absolute URL to call. Example: https://example.com/api
		mutable FString Url;

		// Verb to use. Default: GET
		mutable EHttpMethod Method = EHttpMethod::Get;

		// Optional request headers. Keys are case-insensitive by HTTP standard.
		mutable TMap<FString, FString> Headers;

		// Optional Content-Type. If set and not already provided in Headers, it will be added.
		mutable FString ContentType;

		// Optional request body. If empty, no body is sent.
		mutable TArray<uint8> Body;

		// Timeout in seconds. <= 0 means use engine default.
		mutable float TimeoutSeconds = 0.f;

		// Builder pattern methods for chained construction

		// Sets the URL and returns a reference for chaining.
		FHttpRequest& With_Url(const FString& InUrl)
		{
			Url = InUrl;
			return *this;
		}

		// Sets the HTTP method and returns a reference for chaining.
		FHttpRequest& With_Method(EHttpMethod InMethod)
		{
			Method = InMethod;
			return *this;
		}

		// Adds a single header and returns a reference for chaining.
		FHttpRequest& With_Header(const FString& Key, const FString& Value)
		{
			Headers.Add(Key, Value);
			return *this;
		}

		// Sets/merges multiple headers and returns a reference for chaining.
		FHttpRequest& With_Headers(const TMap<FString, FString>& InHeaders)
		{
			Headers.Append(InHeaders);
			return *this;
		}

		// Sets the Content-Type and returns a reference for chaining.
		FHttpRequest& With_ContentType(const FString& InContentType)
		{
			ContentType = InContentType;
			return *this;
		}

		// Sets the request body (copy) and returns a reference for chaining.
		FHttpRequest& With_Body(const TArray<uint8>& InBody)
		{
			Body = InBody;
			return *this;
		}

		// Sets the request body (move) and returns a reference for chaining.
		FHttpRequest& With_Body(TArray<uint8>&& InBody)
		{
			Body = MoveTemp(InBody);
			return *this;
		}

		// Sets the timeout in seconds and returns a reference for chaining.
		FHttpRequest& With_Timeout(float InTimeoutSeconds)
		{
			TimeoutSeconds = InTimeoutSeconds;
			return *this;
		}
	};

	// Response data for HTTP calls.
	struct BANETTETRANSPORT_API FHttpResponse
	{
		// Final URL (after redirects if any, according to the HTTP module behavior).
		FString Url;

		// HTTP status code. 0 means no valid response was received.
		int32 StatusCode = 0;

		// Response headers.
		TMap<FString, FString> Headers;

		// Response payload.
		TArray<uint8> Body;

		// Parsed/echoed content type if present.
		FString ContentType;

		// Whether the engine reported a successful connection and a response object was received.
		bool bSucceeded = false;
	};

	static FString ToVerb(const EHttpMethod Method);

	class BANETTETRANSPORT_API FHttpService : public TService<FHttpRequest, FHttpResponse>
	{
	public:
		virtual ~FHttpService() override = default;

		// Perform an HTTP call using Unreal's HTTP module.
		virtual UE5Coro::TCoroutine<TResult<FHttpResponse>> Call(
			const FHttpRequest& Request) override;
	};
}
