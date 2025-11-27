// Copyright 2019-Present tarnishablec. All Rights Reserved.


#include "BanetteTestLibrary.h"
#include "BanetteKit/Layers/RetryLayer.h"
#include "BanetteKit/Layers/ExtractLayer.h"
#include "BanetteTransport/Http/HttpService.h"
#include "Dom/JsonObject.h"

using namespace Banette::Transport::Http;
using namespace Banette::Pipeline;
using namespace Banette::Kit;

FVoidCoroutine UBanetteTestLibrary::Test(FJsonObjectWrapper& Json, FLatentActionInfo LatentInfo)
{
	const auto HttpService = MakeShared<FHttpService>();

	TExtractLayer<FHttpService> ExtractLayer;

	ExtractLayer.Register(
		"application/json",
		[](const TArray<uint8>& Bytes)
		{
			const FString JsonString = FString(
				StringCast<TCHAR>(reinterpret_cast<const char*>(Bytes.GetData())).Get()
			);

			TSharedPtr<FJsonValue> OutJson;
			const TSharedRef<TJsonReader<>> Reader =
				TJsonReaderFactory<>::Create(JsonString);

			UE_LOG(LogTemp, Warning, TEXT("JSON: [%s]"), *JsonString);

			FJsonSerializer::Deserialize(Reader, OutJson);
			
			return OutJson;
		});

	ExtractLayer.Register(
		"text/plain",
		[](const TArray<uint8>& Bytes)
		{
			if (Bytes.Num() == 0)
			{
				return MakeShared<FString>();
			}

			const FString Str = FString(
				StringCast<TCHAR>(reinterpret_cast<const char*>(Bytes.GetData())).Get()
			);
			return MakeShared<FString>(Str);
		}
	);

	const TRetryLayer<FHttpService> RetryLayer({
		.MaxAttempts = 5,
		.DelayBetweenRetries = 0.5f,
	});

	const auto WrappedService =
		TServiceBuilder<>::New(HttpService)
		.Layer(RetryLayer)
		.Layer(ExtractLayer)
		.Build();

	FHttpRequest Request;
	Request.Url = TEXT("https://httpbin.org/json");
	Request.Method = EHttpMethod::Get;

	if (auto Result = co_await WrappedService->Call(Request); Result.IsValid())
	{
		const auto& Response = Result.GetValue();
		const auto JsonContent = Response.GetContent<FJsonObject>();

		if (JsonContent.IsValid())
		{
			Json.JsonObject = JsonContent;
		}
	}
	else
	{
		UE_LOG(LogTemp, Error, TEXT("HTTP Request failed after retries"));
	}

	co_return;
}
