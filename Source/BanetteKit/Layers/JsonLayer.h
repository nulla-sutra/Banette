// Copyright 2019-Present tarnishablec. All Rights Reserved.

#pragma once

#include "CoreMinimal.h"
#include "Banette.h"
#include "BanetteTransport/Http/HttpService.h"
#include "Dom/JsonValue.h"
#include "Serialization/JsonReader.h"
#include "Serialization/JsonSerializer.h"
#include "UE5Coro.h"

namespace Banette::Kit
{
	using namespace Banette::Core;
	using namespace Banette::Transport::Http;

	/// Container for JSON body data, holding both raw bytes and parsed JSON.
	///
	/// The raw bytes are always preserved, even if JSON parsing fails.
	/// The JsonValue will be null if the body could not be parsed as valid JSON.
	struct FJsonBody
	{
		/// Original raw response bytes.
		TArray<uint8> RawBytes;

		/// Parsed JSON value. May be null if parsing failed.
		TSharedPtr<FJsonValue> JsonValue;
	};

	/// HTTP response with JSON body.
	///
	/// Similar to FHttpResponse but with FJsonBody instead of TArray<uint8>.
	struct FHttpJsonResponse
	{
		/// Final URL (after redirects if any).
		FString Url;

		/// HTTP status code. 0 means no valid response was received.
		int32 StatusCode = 0;

		/// Response headers.
		TMap<FString, FString> Headers;

		/// Response body containing both raw bytes and parsed JSON.
		FJsonBody Body;

		/// Parsed/echoed content type if present.
		FString ContentType;

		/// Whether the engine reported a successful connection.
		bool bSucceeded = false;

		TSharedPtr<FJsonValue> BodyToJson() const { return Body.JsonValue; }
	};

	/// Service type that returns HTTP responses with JSON-parsed bodies.
	class FHttpJsonService : public TService<FHttpRequest, FHttpJsonResponse>
	{
	public:
		virtual ~FHttpJsonService() override = default;
	};

	/// Layer that transforms FHttpService into FHttpJsonService.
	///
	/// This layer wraps an FHttpService and automatically parses the response
	/// body as JSON, constructing an FHttpJsonResponse with both raw bytes
	/// and parsed JSON data.
	///
	/// Usage example:
	/// @code
	/// using namespace Banette::Pipeline;
	/// using namespace Banette::Transport::Http;
	/// using namespace Banette::Kit;
	///
	/// TSharedRef<FHttpService> Base = MakeShared<FHttpService>();
	/// FJsonLayer JsonLayer;
	///
	/// auto Builder = TServiceBuilder<>::New(Base)
	///     .Layer(JsonLayer);
	///
	/// TSharedRef<FHttpJsonService> JsonService = Builder.Build();
	///
	/// // Now responses will have Body as FJsonBody with both raw bytes and parsed JSON
	/// @endcode
	class FJsonLayer : public TLayer<FHttpService, FHttpJsonService>
	{
	public:
		FJsonLayer() = default;

		virtual TSharedRef<FHttpJsonService> Wrap(TSharedRef<FHttpService> Inner) const override
		{
			return MakeShared<FJsonService>(Inner);
		}

		virtual ~FJsonLayer() override = default;

	private:
		/// Internal service that forwards requests to the inner FHttpService
		/// and converts responses to FHttpJsonResponse with parsed JSON.
		class FJsonService : public FHttpJsonService
		{
		public:
			explicit FJsonService(const TSharedRef<FHttpService>& InInner)
				: InnerService(InInner)
			{
			}

			virtual UE5Coro::TCoroutine<TResult<FHttpJsonResponse>> Call(
				const FHttpRequest& Request) override
			{
				// Forward the request to the inner HTTP service
				auto Result = co_await InnerService->Call(Request);

				// If the request failed, propagate the error
				if (!Result.IsValid())
				{
					co_return MakeError(Result.GetError());
				}

				// Convert FHttpResponse to FHttpJsonResponse
				const FHttpResponse& HttpResponse = Result.GetValue();
				FHttpJsonResponse JsonResponse;

				JsonResponse.Url = HttpResponse.Url;
				JsonResponse.StatusCode = HttpResponse.StatusCode;
				JsonResponse.Headers = HttpResponse.Headers;
				JsonResponse.ContentType = HttpResponse.ContentType;
				JsonResponse.bSucceeded = HttpResponse.bSucceeded;

				// Preserve the raw bytes
				JsonResponse.Body.RawBytes = HttpResponse.Body;

				// Attempt to parse the body as JSON
				JsonResponse.Body.JsonValue = ParseJsonFromBytes(HttpResponse.Body);

				co_return MakeValue(JsonResponse);
			}

		private:
			TSharedRef<FHttpService> InnerService;

			/// Attempts to parse JSON from raw bytes.
			/// Returns nullptr if parsing fails.
			static TSharedPtr<FJsonValue> ParseJsonFromBytes(const TArray<uint8>& Bytes)
			{
				if (Bytes.Num() == 0)
				{
					return nullptr;
				}

				// Convert bytes to string with explicit length to avoid buffer overreads
				// Ensure null-termination by creating an ANSICHAR array with explicit null terminator
				TArray<ANSICHAR> NullTerminatedBytes;
				NullTerminatedBytes.SetNumUninitialized(Bytes.Num() + 1);
				FMemory::Memcpy(NullTerminatedBytes.GetData(), Bytes.GetData(), Bytes.Num());
				NullTerminatedBytes[Bytes.Num()] = '\0';

				const FString JsonString = FString(
					StringCast<TCHAR>(NullTerminatedBytes.GetData()).Get()
				);

				// Check if the JSON string starts with '[' (array) or '{' (object)
				const FString TrimmedJson = JsonString.TrimStartAndEnd();
				if (TrimmedJson.IsEmpty())
				{
					return nullptr;
				}

				const TSharedRef<TJsonReader<>> Reader = TJsonReaderFactory<>::Create(JsonString);

				// Try to parse as array if it starts with '['
				if (TrimmedJson.StartsWith(TEXT("[")))
				{
					TArray<TSharedPtr<FJsonValue>> JsonArray;
					if (FJsonSerializer::Deserialize(Reader, JsonArray))
					{
						// Wrap the array in an FJsonValueArray with correct Type
						return MakeShared<FJsonValueArray>(JsonArray);
					}
					return nullptr;
				}

				// Parse as object or other JSON value
				TSharedPtr<FJsonValue> JsonValue;
				if (FJsonSerializer::Deserialize(Reader, JsonValue))
				{
					return JsonValue;
				}

				// Parsing failed; return nullptr
				return nullptr;
			}
		};
	};
}
