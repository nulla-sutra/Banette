// Copyright 2019-Present tarnishablec. All Rights Reserved.

#pragma once
#include "Banette.h"
#include "Containers/Map.h"
#include "Containers/Array.h"


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

	class BANETTETRANSPORT_API FHttpService : public TService<FHttpRequest, FHttpResponse>
	{
	public:
		// Perform an HTTP call using Unreal's HTTP module.
		virtual UE5Coro::TCoroutine<TValueOrError<FHttpResponse, UE::UnifiedError::FError>> Call(
			const FHttpRequest& Request) override;
	};
}
