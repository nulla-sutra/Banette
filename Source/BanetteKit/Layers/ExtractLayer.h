// Copyright 2019-Present tarnishablec. All Rights Reserved.

#pragma once

#include "CoreMinimal.h"
#include "Banette.h"

// A generic, type-safe extraction layer for Banette.

namespace Banette::Kit
{
	using namespace Banette::Core;

	using FExtractor = TFunction<TSharedPtr<void>(const TArray<uint8>&)>;


	template <typename ResponseT>
	struct TExtractable
	{
		static void GetBytes(const ResponseT& Response, TArray<uint8>& InBytes)
		{
			unimplemented()
		};

		static FString GetContentType(const ResponseT& Response)
		{
			unimplemented()
			return TEXT("");
		}
	};

	template <typename BaseResponseT>
	struct TExtractedResponse : TTuple<BaseResponseT, TMap<FString, FExtractor>>
	{
		template <typename T>
		TSharedPtr<T> GetContent() const
		{
			auto& Extractors = this->template Get<1>();
			auto& Response = this->template Get<0>();
			TArray<uint8> Bytes;
			TExtractable<BaseResponseT>::GetBytes(Response, Bytes);
			const auto ContentType = TExtractable<BaseResponseT>::GetContentType(Response);
			if (const auto Extractor = Extractors.Find(ContentType))
			{
				const auto Result = (*Extractor)(Bytes);
				return StaticCastSharedPtr<T>(Result);
			}

			return nullptr;
		}
	};


	template <CService InService, typename OutService = TService<
		typename InService::RequestType, TExtractedResponse<typename InService::ResponseType>>>
	class TExtractLayer : public TLayer<InService, OutService>
	{
	protected:
		TMap<FString, FExtractor> Extractors;

	public:
		virtual ~TExtractLayer() override = default;

		TExtractLayer& Register(const FString& TypeKey, const FExtractor& Extractor)
		{
			Extractors.Add(TypeKey, Extractor);
			return *this;
		}

		virtual TSharedRef<OutService> Wrap(TSharedRef<InService> Inner) override
		{
			return MakeShared<FExtractService>(Inner, Extractors);
		}

		class FExtractService : public TService<typename InService::RequestType, TExtractedResponse<typename
			                                        InService::ResponseType>>
		{
		public:
			explicit FExtractService(TSharedRef<InService> InInnerService,
			                         const TMap<FString, FExtractor>& InExtractors)
				: InnerService(InInnerService),
				  SavedExtractors(InExtractors)
			{
			}

			virtual UE5Coro::TCoroutine<TResult<typename FExtractService::ResponseType>>
			Call(const FExtractService::RequestType& Request) override
			{
				const TResult<typename InService::ResponseType> Result = co_await InnerService->Call(Request);

				if (!Result.IsValid())
				{
					co_return MakeError(Result.GetError());
				}

				const auto Raw = Result.GetValue();

				co_return MakeValue(MakeTuple(Raw, SavedExtractors));
			}

		protected:
			TSharedRef<InService> InnerService;
			const TMap<FString, FExtractor> SavedExtractors;
		};

		friend class FExtractService;
	};
}

