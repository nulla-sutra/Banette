// All project comments must be in English per repo policy.
// ReSharper disable CppUE4CodingStandardNamingViolationWarning
#pragma once

#include "CoreMinimal.h"
#include "BanetteService.h"

namespace Banette
{
	template <typename RequestT, typename ResponseT>
	class TLayer
	{
	public:
		virtual ~TLayer() = default;

		virtual TSharedRef<TService<RequestT, ResponseT>> Wrap(
			TServiceRef<RequestT, ResponseT> Inner) = 0;
	};


	template <typename RequestT, typename ResponseT>
	class TBanetteServiceChain
	{
	public:
		TBanetteServiceChain& Layer(
			TSharedRef<TLayer<RequestT, ResponseT>> InLayer)
		{
			Layers.Add(InLayer);
			return *this;
		}

		TSharedRef<TService<RequestT, ResponseT>>
		Build(TSharedRef<TService<RequestT, ResponseT>> BaseService) const
		{
			TSharedRef<TService<RequestT, ResponseT>> Current = BaseService;

			for (int32 i = Layers.Num() - 1; i >= 0; --i)
			{
				Current = Layers[i]->Wrap(Current);
			}

			return Current;
		}

	private:
		TArray<TSharedRef<TLayer<RequestT, ResponseT>>> Layers;
	};

	template <typename RequestT, typename ResponseT>
	using TLayerRef = TSharedRef<TLayer<RequestT, ResponseT>>;
}
