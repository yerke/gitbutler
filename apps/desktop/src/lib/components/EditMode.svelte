<script lang="ts">
	import DecorativeSplitView from './DecorativeSplitView.svelte';
	import ProjectNameLabel from '../shared/ProjectNameLabel.svelte';
	import newProjectSvg from '$lib/assets/illustrations/new-project.svg?raw';
	import { Project } from '$lib/backend/projects';
	import { ModeService, type EditModeMetadata } from '$lib/modes/service';
	import { getContext } from '$lib/utils/context';
	import Button from '@gitbutler/ui/Button.svelte';

	interface Props {
		editModeMetadata: EditModeMetadata;
	}

	const { editModeMetadata }: Props = $props();

	const project = getContext(Project);
	const modeService = getContext(ModeService);

	let modeServiceSaving = $state<'inert' | 'loading' | 'completed'>('inert');

	async function abort() {
		modeServiceSaving = 'loading';

		await modeService.abortEditAndReturnToWorkspace();

		modeServiceSaving = 'completed';
	}

	async function save() {
		modeServiceSaving = 'loading';

		await modeService.saveEditAndReturnToWorkspace();

		modeServiceSaving = 'completed';
	}
</script>

<DecorativeSplitView img={newProjectSvg}>
	<div class="switchrepo">
		<div class="project-name">
			<ProjectNameLabel projectName={project?.title} />
		</div>
		<p class="switchrepo__title text-18 text-body text-bold">
			You are currently editing commit <span class="code-string">
				{editModeMetadata.commitOid.slice(0, 7)}
			</span>
		</p>

		<p class="switchrepo__message text-13 text-body">
			Please do not make any commits whilst in edit mode. To leave edit mode, use the provided
			actions.
		</p>

		<div class="switchrepo__actions">
			<Button style="ghost" outline onclick={abort} loading={modeServiceSaving === 'loading'}>
				Cancel changes
			</Button>
			<Button
				style="pop"
				kind="solid"
				icon="undo-small"
				onclick={save}
				loading={modeServiceSaving === 'loading'}
			>
				Save and exit
			</Button>
		</div>
	</div>
</DecorativeSplitView>

<style lang="postcss">
	.project-name {
		margin-bottom: 12px;
	}

	.switchrepo__title {
		color: var(--clr-scale-ntrl-30);
		margin-bottom: 12px;
	}

	.switchrepo__message {
		color: var(--clr-scale-ntrl-50);
		margin-bottom: 20px;
	}
	.switchrepo__actions {
		display: flex;
		gap: 8px;
		padding-bottom: 24px;
		flex-wrap: wrap;
		justify-content: flex-end;
	}
</style>
