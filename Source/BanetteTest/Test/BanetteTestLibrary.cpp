// Copyright 2019-Present tarnishablec. All Rights Reserved.


#include "BanetteTestLibrary.h"
#include "BanetteKit/Layers/RetryLayer.h"
#include "BanetteKit/Layers/ExtractLayer.h"
#include "BanetteTransport/Http/BanetteHttp.h"
#include "Dom/JsonObject.h"

using namespace Banette::Transport::Http;
using namespace Banette::Pipeline;
using namespace Banette::Kit;

FVoidCoroutine UBanetteTestLibrary::Test(FLatentActionInfo LatentInfo)
{
	const auto HttpService = MakeShared<FHttpService>();

	TRetryLayer<FHttpService> RetryLayer({
		.MaxAttempts = 5,
		.DelayBetweenRetries = 0.5f,
		.Challenge = [](const FHttpService::ResponseType& Response)
		{
			return Response.bSucceeded;
		}
	});

	const auto WrappedService =
		TServiceBuilder<>::New(HttpService)
		.Layer(RetryLayer)
		.Build();

	FHttpRequest Request;
	Request.Url = TEXT("https://httpbin.org/get");
	Request.Method = EHttpMethod::Get;

	if (auto Result = co_await WrappedService->Call(Request); Result.IsValid())
	{
		const auto& Response = Result.GetValue();
		UE_LOG(LogTemp, Log, TEXT("HTTP Request succeeded with status code: %d"), Response.StatusCode);
	}
	else
	{
		UE_LOG(LogTemp, Error, TEXT("HTTP Request failed after retries"));
	}

	co_return;
}

template <>
struct TExtractable<FHttpResponse>
{
	static const TArray<uint8>& GetBytes(const FHttpResponse& Response)

	{
		return Response.Body;
	};

	static FString GetTypeKey(const FHttpResponse& Response)
	{
		return Response.ContentType;
	}
};

FVoidCoroutine UBanetteTestLibrary::Test2(FLatentActionInfo LatentInfo)
{
	const auto HttpService = MakeShared<FHttpService>();

	TExtractLayer<FHttpService> ExtractLayer;

	ExtractLayer.Register("application/json",
	                      [](const TArray<uint8>& Bytes)
	                      {
		                      auto Json = MakeShared<FJsonObject>();
		                      Json->SetNumberField("test", 123);
		                      return Json;
	                      });

	ExtractLayer.Register("text/plain",
	                      [](const TArray<uint8>& Bytes)
	                      {
		                      return MakeShared<FString>();
	                      }
	);


	const auto WrappedService =
		TServiceBuilder<>::New(HttpService)
		.Layer(ExtractLayer)
		.Build();

	FHttpRequest Request;
	Request.Url = TEXT("https://httpbin.org/get");
	Request.Method = EHttpMethod::Get;

	if (auto Result = co_await WrappedService->Call(Request); Result.IsValid())
	{
		const auto& Response = Result.GetValue();
		const auto Json = Response.GetContent<FJsonObject>();

		if (Json.IsValid())
		{
			check(0);
		}
	}
	else
	{
		UE_LOG(LogTemp, Error, TEXT("HTTP Request failed after retries"));
	}

	co_return;
}
