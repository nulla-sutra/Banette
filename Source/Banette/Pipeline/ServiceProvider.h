// Copyright 2019-Present tarnishablec. All Rights Reserved.

#pragma once

#include "CoreMinimal.h"
#include "Banette/Core/Service.h"

namespace Banette::Pipeline
{
	using namespace Banette::Core;

	template <CService T>
	struct TServiceProvider
	{
		static inline TSharedPtr<T> Service;

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

			Service = TServiceProvider<T>::BuildService();
			return Service;
		};
	};
}
