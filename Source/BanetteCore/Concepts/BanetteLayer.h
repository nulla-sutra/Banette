// Copyright 2019-Present tarnishablec. All Rights Reserved.
// ReSharper disable CppUE4CodingStandardNamingViolationWarning
#pragma once

#include "CoreMinimal.h"
#include "BanetteService.h"

namespace Banette
{
	template <typename RequestT, typename ResponseT>
	class ILayer
	{
	public:
		virtual ~ILayer() = default;

		virtual TSharedRef<TService<RequestT, ResponseT>>
		Wrap(TSharedRef<TService<RequestT, ResponseT>> Inner) = 0;
	};


	template <typename RequestT, typename ResponseT>
	class TBanetteServiceChain
	{
	public:
		TBanetteServiceChain& Layer(
			TSharedRef<ILayer<RequestT, ResponseT>> InLayer)
		{
			Layers.Add(InLayer);
			return *this;
		}

		TSharedRef<TService<RequestT, ResponseT>>
		Build(TSharedRef<TService<RequestT, ResponseT>> BaseService) const
		{
			auto Current = BaseService;

			for (int32 i = Layers.Num() - 1; i >= 0; --i)
			{
				Current = Layers[i]->Wrap(Current);
			}

			return Current;
		}

	private:
		TArray<TSharedRef<ILayer<RequestT, ResponseT>>> Layers;
	};

	template <typename RequestT, typename ResponseT>
	using TLayerRef = TSharedRef<ILayer<RequestT, ResponseT>>;
}
