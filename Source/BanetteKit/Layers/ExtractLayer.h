// Copyright 2019-Present tarnishablec. All Rights Reserved.

#pragma once

#include "CoreMinimal.h"
#include "Banette.h"

namespace Banette::Kit
{
	using namespace Banette::Core;

	using FExtractor = TFunction<TSharedPtr<void>(const TArray<uint8>&)>;

	using FExtractorMap = TMap<FString, FExtractor>;

	template <typename ResponseT>
	struct TExtractable
	{
		static const TArray<uint8>& GetBytes(const ResponseT& Response)
		{
			unimplemented()
			static const TArray<uint8> EmptyBytes;
			return EmptyBytes;
		}

		static FString GetTypeKey(const ResponseT& Response)
		{
			unimplemented()
			return "";
		}
	};

	template <typename T>
	concept CExtractable = requires(const T& Type)
	{
		{ TExtractable<T>::GetBytes(Type) } -> std::convertible_to<const TArray<uint8>&>;
		{ TExtractable<T>::GetTypeKey(Type) } -> std::convertible_to<FString>;
	};

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

				// CRTP
				const TArray<uint8>& Bytes = TExtractable<FBaseResponse>::GetBytes(BaseResponse);
				const FString TypeKey = TExtractable<FBaseResponse>::GetTypeKey(BaseResponse);

				TSharedPtr<void> ParsedContent = nullptr;

				if (const FExtractor* Found = Extractors->Find(TypeKey))
				{
					if (Bytes.Num() > 0)
					{
						ParsedContent = (*Found)(Bytes);
					}
					co_return MakeValue(MakeTuple(MoveTemp(BaseResponse), MoveTemp(ParsedContent)));
				}

				co_return TResult<FWrapperResponse>();
			}

		private:
			TSharedRef<InService> InnerService;
			TSharedRef<const FExtractorMap> Extractors;
		};
	};
}
