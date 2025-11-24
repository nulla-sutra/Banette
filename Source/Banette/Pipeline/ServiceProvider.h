// Copyright 2019-Present tarnishablec. All Rights Reserved.

#pragma once

#include "CoreMinimal.h"

namespace Banette::Pipeline
{
	template <CService T>
	struct TServiceProvider
	{
		static TSharedPtr<T> GetService()
		{
			unimplemented();
			return nullptr;
		};
	};
}
