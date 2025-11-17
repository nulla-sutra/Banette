// Copyright 2019-Present tarnishablec. All Rights Reserved.

#pragma once

#include "CoreMinimal.h"
#include "HttpModule.h"
#include "Templates/SharedPointer.h"
#include "UE5Coro.h"
#include "Concepts/BanetteService.h"

#include "Interfaces/IHttpResponse.h"

namespace Banette::Http
{
	/**
	 * Minimal HTTP request/response used by Banette::Http::FHttpTransport
	 */
	struct FRequest
	{
		FString Url;
		FString Method = TEXT("GET"); // GET, POST, PUT, DELETE, Patch
		TMap<FString, FString> Headers;
		TArray<uint8> Body; // raw body bytes (UTF-8 JSON typical)
		float TimeoutSeconds = 30.0f; // optional per-request timeout
	};

	struct FResponse
	{
		int32 StatusCode = 0;
		TMap<FString, FString> Headers;
		TArray<uint8> Body;
		bool bIsError = false;
		FString ErrorMessage;
	};


	class BANETTETRANSPORT_API FHttpTransport : public TService<FRequest, FResponse>
	{
	public:
		FHttpTransport() = default;
		virtual ~FHttpTransport() override = default;

		// TBanetteService override
		virtual UE5Coro::TCoroutine<FResponse> Call(const FRequest& Request) override
		{
			// Build low-level IHttpRequest
			const auto HttpReq = BuildHttpRequest(Request);

			// Process asynchronously using UE5Coro::Http instead of IHttpRequest::ProcessRequest
			using namespace UE5Coro::Http;
			const auto ResultTuple = co_await ProcessAsync(HttpReq);
			const FHttpResponsePtr& RespPtr = ResultTuple.Get<0>();
			const bool bWasSuccessful = ResultTuple.Get<1>();

			FResponse Out;

			if (!bWasSuccessful || !RespPtr.IsValid())
			{
				Out.bIsError = true;
				Out.ErrorMessage = TEXT("Network error or no response");
				co_return Out;
			}

			Out.StatusCode = RespPtr->GetResponseCode();

			// Parse headers (GetAllHeaders returns an array of "Key: Value" strings)
			const TArray<FString> HeaderStrings = RespPtr->GetAllHeaders();
			for (const FString& H : HeaderStrings)
			{
				FString Key, Value;
				if (H.Split(TEXT(":"), &Key, &Value))
				{
					Key.TrimStartAndEndInline();
					Value.TrimStartAndEndInline();
					Out.Headers.Add(Key, Value);
				}
			}

			// Copy body (as bytes)
			const TArray<uint8>& Content = RespPtr->GetContent();
			Out.Body = Content;

			// Mark non-2xx as error (caller can inspect StatusCode)
			if (Out.StatusCode < 200 || Out.StatusCode >= 300)
			{
				Out.bIsError = true;
				Out.ErrorMessage = FString::Printf(TEXT("HTTP error %d"), Out.StatusCode);
			}
			else
			{
				Out.bIsError = false;
			}

			co_return Out;
		}

	private:
		// Helper to build the IHttpRequestPtr from our FBanetteRequest
		static TSharedRef<IHttpRequest, ESPMode::ThreadSafe> BuildHttpRequest(const FRequest& Req)
		{
			FHttpModule& HttpModule = FHttpModule::Get();
			TSharedRef<IHttpRequest, ESPMode::ThreadSafe> HttpRequest = HttpModule.CreateRequest();

			HttpRequest->SetURL(Req.Url);
			HttpRequest->SetVerb(Req.Method);
			HttpRequest->SetTimeout(Req.TimeoutSeconds);

			// Set headers
			for (const auto& It : Req.Headers)
			{
				HttpRequest->SetHeader(It.Key, It.Value);
			}

			// Set content
			if (!Req.Body.IsEmpty())
			{
				// Use SetContent to set a binary body
				HttpRequest->SetContent(Req.Body);
			}

			return HttpRequest;
		}
	};
}
