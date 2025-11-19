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
		FString Url;

		// Verb to use. Default: GET
		EHttpMethod Method = EHttpMethod::Get;

		// Optional request headers. Keys are case-insensitive by HTTP standard.
		TMap<FString, FString> Headers;

		// Optional Content-Type. If set and not already provided in Headers, it will be added.
		FString ContentType;

		// Optional request body. If empty, nobody is sent.
		TArray<uint8> Body;

		// Timeout in seconds. <= 0 means use engine default.
		float TimeoutSeconds = 0.f;
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
