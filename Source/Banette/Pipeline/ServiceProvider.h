// Copyright 2019-Present tarnishablec. All Rights Reserved.

#pragma once

#include "CoreMinimal.h"
#include "Banette/Core/Service.h"

namespace Banette::Pipeline
{
	template <CService T, typename ProviderT = void>
	struct TServiceProvider
	{
		static inline TSharedPtr<T> Service = nullptr;

		static TSharedPtr<T> BuildService()
		{
			unimplemented();
			return nullptr;
		};

		static TSharedPtr<T> GetService()
		{
			if (Service.IsValid())
			{
				return Service;
			}

			if constexpr (!std::is_same_v<ProviderT, void>)
			{
				Service = ProviderT::BuildService();
			}
			else
			{
				Service = BuildService();
			}

			return Service;
		};
	};
}
