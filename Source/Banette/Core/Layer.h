// Copyright 2019-Present tarnishablec. All Rights Reserved.
#pragma once

#include "CoreMinimal.h"
#include "Definition.h"
#include "Service.h"

namespace Banette::Core
{
	template <CService InServiceT, CService OutService>
	class BANETTE_API TLayer
	{
		using InServiceType = InServiceT;
		using OutServiceType = OutService;

	public:
		virtual ~TLayer() = default;

		virtual TSharedRef<OutService> Wrap(TSharedRef<InServiceT> Inner) = 0;
	};

	template <typename S>
	concept CLayer =
		std::is_base_of_v<
			TLayer<typename S::InServiceType, typename S::OutServiceType>,
			S>;
}
