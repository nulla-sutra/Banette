// Copyright 2019-Present tarnishablec. All Rights Reserved.

#pragma once

#include "CoreMinimal.h"
#include "Banette.h"
#include "BanetteTransport/Http/HttpService.h"
#include "UE5Coro.h"

namespace Banette::Kit
{
	using namespace Banette::Core;
	using namespace Banette::Transport::Http;

	/// Layer that injects HTTP headers into outgoing requests.
	///
	/// This layer wraps an FHttpService and merges configured headers into each
	/// request before forwarding it to the inner service.
	///
	/// Usage example:
	/// @code
	/// using namespace Banette::Transport::Http;
	/// using namespace Banette::Kit;
	///
	/// TSharedRef<FHttpService> Base = MakeShared<FHttpService>();
	/// FInjectHeaderLayer Layer({{TEXT("X-Auth"), TEXT("Token")}}, /* bOverrideExisting = */ true);
	/// TSharedRef<FHttpService> WithHeaders = Layer.Wrap(Base);
	/// @endcode
	class FInjectHeaderLayer : public TLayer<FHttpService, FHttpService>
	{
	public:
		/**
		 * Construct a header-injection layer.
		 * @param InHeaders       Headers to inject into each request.
		 * @param bInOverrideExisting If true, injected headers override existing request headers.
		 *                            If false, headers are only added if not already present.
		 */
		explicit FInjectHeaderLayer(
			const TMap<FString, FString>& InHeaders = {},
			const bool bInOverrideExisting = false)
			: Headers(InHeaders)
			  , bOverrideExisting(bInOverrideExisting)
		{
		}

		/**
		 * Add a header to the injection set. Returns *this for chaining.
		 */
		FInjectHeaderLayer& AddHeader(const FString& Name, const FString& Value)
		{
			Headers.Add(Name, Value);
			return *this;
		}

		virtual TSharedRef<FHttpService> Wrap(TSharedRef<FHttpService> Inner) const override
		{
			return MakeShared<FInjectHeaderService>(Inner, Headers, bOverrideExisting);
		}

		virtual ~FInjectHeaderLayer() override = default;

	private:
		TMap<FString, FString> Headers;
		bool bOverrideExisting;

		/**
		 * Internal service wrapper that performs header injection on each request.
		 */
		class FInjectHeaderService : public FHttpService
		{
		public:
			FInjectHeaderService(
				const TSharedRef<FHttpService>& InInner,
				const TMap<FString, FString>& InHeaders,
				const bool bInOverrideExisting)
				: InnerService(InInner)
				  , Headers(InHeaders)
				  , bOverrideExisting(bInOverrideExisting)
			{
			}

			virtual UE5Coro::TCoroutine<TResult<FHttpResponse>> Call(
				const FHttpRequest& Request) override
			{
				// Create a copy of the request to inject headers
				FHttpRequest ModifiedRequest = Request;

				// Merge configured headers into the request
				for (const auto& Kvp : Headers)
				{
					if (bOverrideExisting)
					{
						// Always set/replace the header
						ModifiedRequest.Headers.FindOrAdd(Kvp.Key) = Kvp.Value;
					}
					else
					{
						// Only add if the key does not already exist
						if (!ModifiedRequest.Headers.Contains(Kvp.Key))
						{
							ModifiedRequest.Headers.Add(Kvp.Key, Kvp.Value);
						}
					}
				}

				// Forward the modified request to the inner service
				co_return co_await InnerService->Call(ModifiedRequest);
			}

		private:
			TSharedRef<FHttpService> InnerService;
			TMap<FString, FString> Headers;
			bool bOverrideExisting;
		};
	};
}
