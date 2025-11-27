// Copyright 2019-Present tarnishablec. All Rights Reserved.

#pragma once

#include "CoreMinimal.h"
#include "Banette.h"
#include "BanetteTransport/Http/HttpService.h"

namespace Banette::Kit
{
	using namespace Banette::Core;

	using FExtractor = TFunction<TSharedPtr<void>(const TArray<uint8>&)>;

	using FExtractorMap = TMap<FString, FExtractor>;

	/**
	 * TExtractable is a traits template that must be specialized for each response type.
	 * 
	 * The primary template is explicitly deleted to ensure a compile-time error if used
	 * with an unsupported response type. To support a new response type, provide a
	 * specialization that defines:
	 *   - static const TArray<uint8>& GetBytes(const ResponseT& Response);
	 *   - static FString GetTypeKey(const ResponseT& Response);
	 * 
	 * See the FHttpResponse specialization at the bottom of this file for an example.
	 */
	template <typename ResponseT>
	struct TExtractable
	{
		static_assert(sizeof(ResponseT) == 0,
		              "TExtractable<T> is not specialized for this type. "
		              "You must provide a specialization with GetBytes and GetTypeKey methods.");

		static const TArray<uint8>& GetBytes(const ResponseT& Response) = delete;
		static FString GetTypeKey(const ResponseT& Response) = delete;
	};

	template <typename T>
	concept CExtractable = requires(const T& Type)
	{
		{ TExtractable<T>::GetBytes(Type) } -> std::convertible_to<const TArray<uint8>&>;
		{ TExtractable<T>::GetTypeKey(Type) } -> std::convertible_to<FString>;
	};

	/**
	 * TExtractedResponse wraps the base response along with optionally extracted content.
	 * 
	 * Use GetBase() to access the original response.
	 * Use GetContent<T>() to access the parsed content, where T is the expected type.
	 * 
	 * Note: GetContent<T>() may return a null TSharedPtr<T> if:
	 *   - No extractor was registered for the response's content type.
	 *   - The response body was empty.
	 *   - The extractor returned nullptr (e.g., parse failure).
	 * 
	 * Callers MUST check the returned pointer for validity before use.
	 */
	template <typename BaseResponseT>
	struct TExtractedResponse : TTuple<BaseResponseT, TSharedPtr<void>>
	{
		const BaseResponseT& GetBase() const { return this->template Get<0>(); }

		template <typename T>
		TSharedPtr<T> GetContent() const
		{
			return StaticCastSharedPtr<T>(this->template Get<1>());
		}
	};

	/**
	 * TExtractLayer is a service layer that extracts typed content from responses.
	 * 
	 * Register extractors for specific content types using Register(TypeKey, Extractor).
	 * When a response is received:
	 *   1. If the inner service returns an error, that error is propagated.
	 *   2. If successful, the response is wrapped in TExtractedResponse.
	 *   3. Content extraction is attempted only if Bytes.Num() > 0 and an extractor
	 *      is registered for the response's TypeKey.
	 *   4. The result is ALWAYS a valid TResult when the inner call succeeded, even if
	 *      extraction was skipped or failed. Callers must check GetContent<T>() for nullptr.
	 * 
	 * This design ensures callers never crash due to missing extractors or parse failures.
	 * The layer provides a stable contract where nullptr content is a valid, expected outcome.
	 */
	template <CService InService>
		requires CExtractable<typename InService::ResponseType>
	class TExtractLayer : public TLayer<
			InService,
			TService<
				typename InService::RequestType,
				TExtractedResponse<typename InService::ResponseType>
			>
		>
	{
	public:
		using FBaseResponse = InService::ResponseType;
		using FWrapperResponse = TExtractedResponse<typename InService::ResponseType>;
		using FOutService = TService<typename InService::RequestType, FWrapperResponse>;

		TExtractLayer()
			: Extractors(MakeShared<FExtractorMap>())
		{
		}

		TExtractLayer& Register(const FString& TypeKey, const FExtractor& Extractor)
		{
			Extractors->Add(TypeKey, Extractor);
			return *this;
		}

		virtual TSharedRef<FOutService> Wrap(TSharedRef<InService> Inner) const override
		{
			return MakeShared<FExtractService>(Inner, Extractors.ToSharedRef());
		}

	private:
		TSharedPtr<FExtractorMap> Extractors;

		/**
		 * FExtractService wraps the inner service and extracts typed content from responses.
		 * 
		 * Contract:
		 * - If the inner service returns an error, that error is propagated unchanged.
		 * - If the inner service succeeds:
		 *   - A valid TResult<FWrapperResponse> is ALWAYS returned (never a default error).
		 *   - ParsedContent will be nullptr if:
		 *     - No extractor is registered for the response's TypeKey.
		 *     - The response body is empty (Bytes.Num() == 0).
		 *     - The registered extractor returns nullptr (e.g., parse failure).
		 *   - ParsedContent will be non-null only if extraction succeeded.
		 * 
		 * Callers MUST check GetContent<T>() for nullptr before using the result.
		 */
		class FExtractService : public FOutService
		{
		public:
			explicit FExtractService(
				TSharedRef<InService> InInner,
				const TSharedRef<const FExtractorMap>& InExtractors
			)
				: InnerService(InInner)
				  , Extractors(InExtractors)
			{
			}

			virtual UE5Coro::TCoroutine<TResult<FWrapperResponse>>
			Call(const InService::RequestType& Request) override
			{
				auto Result = co_await InnerService->Call(Request);

				if (Result.HasError())
				{
					co_return MakeError(Result.GetError());
				}

				FBaseResponse BaseResponse = Result.StealValue();

				const TArray<uint8>& Bytes = TExtractable<FBaseResponse>::GetBytes(BaseResponse);
				const FString TypeKey = TExtractable<FBaseResponse>::GetTypeKey(BaseResponse);

				TSharedPtr<void> ParsedContent = nullptr;

				// Only attempt extraction if we have bytes and a registered extractor.
				// If either is missing, ParsedContent remains nullptr, but the result is still valid.
				if (Bytes.Num() > 0)
				{
					if (const FExtractor* Found = Extractors->Find(TypeKey))
					{
						// Extractor may return nullptr on parse failure; that's acceptable.
						ParsedContent = (*Found)(Bytes);
					}
				}

				// Always return a valid result when the inner call succeeded.
				// Callers must check GetContent<T>() for nullptr.
				co_return MakeValue(MakeTuple(MoveTemp(BaseResponse), MoveTemp(ParsedContent)));
			}

		private:
			TSharedRef<InService> InnerService;
			TSharedRef<const FExtractorMap> Extractors;
		};
	};
}


template <>
struct Banette::Kit::TExtractable<Banette::Transport::Http::FHttpResponse>
{
	static const TArray<uint8>& GetBytes(const Transport::Http::FHttpResponse& Response)
	{
		return Response.Body;
	};

	static FString GetTypeKey(const Transport::Http::FHttpResponse& Response)
	{
		return Response.ContentType;
	}
};
