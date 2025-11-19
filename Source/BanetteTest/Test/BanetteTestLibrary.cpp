// Copyright 2019-Present tarnishablec. All Rights Reserved.


#include "BanetteTestLibrary.h"
#include "BanetteKit/Layers/RetryLayer.h"
#include "BanetteTransport/Http/BanetteHttp.h"

FVoidCoroutine UBanetteTestLibrary::Test(FLatentActionInfo LatentInfo)
{
	using namespace Banette::Transport::Http;

	const auto HttpService = MakeShared<FHttpService>();

	Banette::Kit::TRetryLayer<FHttpService> RetryLayer({
		.MaxAttempts = 5,
		.DelayBetweenRetries = 0.5f,
		.Challenge = [](const FHttpService::ResponseType& Response)
		{
			return Response.bSucceeded;
		}
	});

	const auto WrappedService =
		Banette::Builder::TServiceBuilder<>::New(HttpService)
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
