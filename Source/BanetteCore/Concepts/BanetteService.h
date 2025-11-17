// Copyright 2019-Present tarnishablec. All Rights Reserved.

#pragma once

#include "CoreMinimal.h"
#include "UE5Coro/Coroutine.h"

namespace Banette
{
	template <typename RequestT, typename ResponseT>
	class BANETTECORE_API TService : public TSharedFromThis<TService<RequestT, ResponseT>>
	{
	public:
		virtual ~TService() = default;

		virtual UE5Coro::TCoroutine<ResponseT> Call(const RequestT& Request) = 0;
	};


	template <typename RequestT, typename ResponseT>
	using TServiceRef = TSharedRef<TService<RequestT, ResponseT>>;
}
