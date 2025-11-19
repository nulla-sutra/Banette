// Copyright 2019-Present tarnishablec. All Rights Reserved.


#include "BanetteTestLibrary.h"
#include "Banette/Pipeline/ServiceBuilder.h"
#include "BanetteKit/Layers/RetryLayer.h"
#include "Http/BanetteHttp.h"

void UBanetteTestLibrary::Test()
{
	using namespace Banette::Transport::Http;

	// 创建一个 HTTP Service
	const auto HttpService = MakeShared<FHttpService>();

	// 配置重试策略
	Banette::Kit::FRetryConfig RetryConfig;
	RetryConfig.MaxAttempts = 5;
	RetryConfig.DelayBetweenRetries = 0.5f;

	// 创建重试 Layer
	Banette::Kit::TRetryLayer<FHttpService> RetryLayer(RetryConfig);

	// 使用 ServiceBuilder 组合它们
	auto WrappedService = Banette::Builder::TServiceBuilder<>::New(HttpService)
	                      .Layer(RetryLayer)
	                      .Build();

	// 现在 WrappedService 具有重试功能！
}
