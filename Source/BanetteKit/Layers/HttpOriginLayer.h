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

	/// Layer that prefixes request URLs with a configured origin (base URL).
	///
	/// This layer wraps an FHttpService and prepends the origin to each
	/// request's URL if the URL does not already start with http:// or https://.
	///
	/// URL concatenation handles trailing/leading slashes:
	/// - origin "https://example.com" + url "/a/b" -> "https://example.com/a/b"
	/// - origin "https://example.com/" + url "a/b" -> "https://example.com/a/b"
	/// - origin "https://example.com/" + url "/a/b" -> "https://example.com/a/b"
	/// - origin "https://example.com" + url "a/b" -> "https://example.com/a/b"
	///
	/// If the origin is empty and the request URL is relative, the layer
	/// returns an InvalidUrl error without calling the inner service.
	///
	/// Usage example:
	/// @code
	/// using namespace Banette::Pipeline;
	/// using namespace Banette::Transport::Http;
	/// using namespace Banette::Kit;
	///
	/// TSharedRef<FHttpService> Base = MakeShared<FHttpService>();
	/// FHttpOriginLayer OriginLayer(TEXT("https://someorigin.com"));
	///
	/// auto Builder = TServiceBuilder<>::New(Base)
	///     .Layer(OriginLayer);
	///
	/// TSharedRef<FHttpService> ServiceWithOrigin = Builder.Build();
	/// @endcode
	class FHttpOriginLayer : public TLayer<FHttpService, FHttpService>
	{
	public:
		/**
		 * Construct an origin-prefixing layer.
		 * @param InOrigin The base URL to prefix (e.g., "https://example.com").
		 */
		explicit FHttpOriginLayer(const FString& InOrigin = FString())
			: Origin(InOrigin)
		{
		}

		virtual TSharedRef<FHttpService> Wrap(TSharedRef<FHttpService> Inner) const override
		{
			return MakeShared<FHttpOriginService>(Inner, Origin);
		}

		virtual ~FHttpOriginLayer() override = default;

	private:
		FString Origin;

		/**
		 * Internal service wrapper that performs URL prefixing on each request.
		 */
		class FHttpOriginService : public FHttpService
		{
		public:
			FHttpOriginService(
				const TSharedRef<FHttpService>& InInner,
				const FString& InOrigin)
				: InnerService(InInner)
				, Origin(InOrigin)
			{
			}

			virtual UE5Coro::TCoroutine<TResult<FHttpResponse>> Call(
				const FHttpRequest& Request) override
			{
				// Check if the URL is already absolute (starts with http:// or https://)
				if (IsAbsoluteUrl(Request.Url))
				{
					// Pass through unchanged
					co_return co_await InnerService->Call(Request);
				}

				// URL is relative; we need to prefix with origin
				if (Origin.IsEmpty())
				{
					// Cannot construct a valid URL without an origin
					co_return MakeError(UE::UnifiedError::Banette::Transport::Http::InvalidUrl::MakeError());
				}

				// Combine origin and relative URL
				FHttpRequest ModifiedRequest = Request;
				ModifiedRequest.Url = CombineUrl(Origin, Request.Url);

				co_return co_await InnerService->Call(ModifiedRequest);
			}

		private:
			TSharedRef<FHttpService> InnerService;
			FString Origin;

			/**
			 * Check if a URL is absolute (starts with http:// or https://).
			 */
			static bool IsAbsoluteUrl(const FString& Url)
			{
				return Url.StartsWith(TEXT("http://"), ESearchCase::IgnoreCase) ||
				       Url.StartsWith(TEXT("https://"), ESearchCase::IgnoreCase);
			}

			/**
			 * Combine origin and relative path, handling trailing/leading slashes.
			 * Examples:
			 * - CombineUrl("https://example.com", "/a/b") -> "https://example.com/a/b"
			 * - CombineUrl("https://example.com/", "a/b") -> "https://example.com/a/b"
			 * - CombineUrl("https://example.com/", "/a/b") -> "https://example.com/a/b"
			 * - CombineUrl("https://example.com", "a/b") -> "https://example.com/a/b"
			 */
			static FString CombineUrl(const FString& BaseOrigin, const FString& RelativePath)
			{
				// Remove trailing slash from origin
				FString NormalizedOrigin = BaseOrigin;
				while (NormalizedOrigin.EndsWith(TEXT("/")))
				{
					NormalizedOrigin.RemoveFromEnd(TEXT("/"));
				}

				// Remove leading slash from path
				FString NormalizedPath = RelativePath;
				while (NormalizedPath.StartsWith(TEXT("/")))
				{
					NormalizedPath.RemoveFromStart(TEXT("/"));
				}

				// Combine with a single slash
				if (NormalizedPath.IsEmpty())
				{
					return NormalizedOrigin;
				}

				return NormalizedOrigin + TEXT("/") + NormalizedPath;
			}
		};
	};
}
