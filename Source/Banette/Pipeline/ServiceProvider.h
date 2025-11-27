// Copyright 2019-Present tarnishablec. All Rights Reserved.

#pragma once

#include "CoreMinimal.h"
#include "Core/Service.h"

namespace Banette::Pipeline
{
	using namespace Banette::Core;

	template <CService T>
	struct TServiceProvider
	{
		virtual ~TServiceProvider() = default;

		static TSharedPtr<T> BuildService()
		{
			static_assert(sizeof(T) == 0,
			              "Banette::Pipeline::GetServiceImpl(TServiceTag<T>) is not specialized "
			              "for this service type T. Please provide an overload in some header.");
			return nullptr;
		}

		static TSharedPtr<T> GetService()
		{
			static TSharedPtr<T> Service = nullptr;

			if (!Service.IsValid())
			{
				Service = BuildService();
				return Service;
			}
			return Service;
		}
	};
}
